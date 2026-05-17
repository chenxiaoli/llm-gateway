#!/usr/bin/env python3
"""Backfill pricing_policy and weighted_tokens in usage_records.

For rows where pricing_policy is NULL, resolves the current pricing policy
from the models table and computes weighted_tokens using the same formula
as the Rust calculate_weighted_tokens function.

Usage:
    python scripts/backfill_pricing_policy.py [--dry-run] [--batch-size 500]
"""

import argparse
import json
import sys
import re
from pathlib import Path

import psycopg2
import psycopg2.extras


# ── Weighted token calculation (mirrors Rust workers.rs) ──────────────────────

def _price_weight(config: dict, field: str, base_price: float) -> float:
    price = config.get(field, 0) or 0
    if isinstance(price, (int, float)) and base_price > 0:
        return price / base_price
    return 0.0


def _weighted_from_config(config: dict, inp: int, out: int, cr: int, cc: int) -> int:
    base = config.get("input_price_1m", 0) or 0
    if base <= 0:
        return inp + out + cr + cc
    w_out = _price_weight(config, "output_price_1m", base)
    w_cr = _price_weight(config, "cache_read_price_1m", base)
    w_cc = _price_weight(config, "cache_creation_price_1m", base)
    return round(inp + out * w_out + cr * w_cr + cc * w_cc)


def _weighted_from_tiered(config: dict, inp: int, out: int) -> int:
    tiers = config.get("tiers")
    if not tiers or not isinstance(tiers, list):
        return inp + out
    first = tiers[0]
    base = first.get("input_price_1m", 0) or 0
    if base <= 0:
        return inp + out
    w_out = _price_weight(first, "output_price_1m", base)
    return round(inp + out * w_out)


def _weighted_from_context_tiered(config: dict, inp: int, out: int, cr: int, cc: int) -> int:
    tiers = config.get("tiers")
    if not tiers or not isinstance(tiers, list):
        return inp + out + cr + cc
    first = tiers[0]
    base = first.get("input_price_1m", 0) or 0
    if base <= 0:
        return inp + out + cr + cc
    w_out = _price_weight(first, "output_price_1m", base)
    w_cr = _price_weight(first, "cache_read_price_1m", base)
    w_cc = _price_weight(first, "cache_creation_price_1m", base)
    return round(inp + out * w_out + cr * w_cr + cc * w_cc)


def calculate_weighted_tokens(policy: dict | None, inp: int, out: int, cr: int, cc: int) -> int:
    if policy is None:
        return inp + out + cr + cc
    config = policy.get("config") or {}
    billing_type = policy.get("billing_type", "per_token")
    match billing_type:
        case "per_token" | "hybrid":
            return _weighted_from_config(config, inp, out, cr, cc)
        case "tiered_token":
            return _weighted_from_tiered(config, inp, out)
        case "context_tiered":
            return _weighted_from_context_tiered(config, inp, out, cr, cc)
        case "per_character":
            return _weighted_from_config(config, inp, out, 0, 0)
        case _:
            return inp + out


# ── Main ──────────────────────────────────────────────────────────────────────

def load_db_url() -> str:
    config_path = Path(__file__).resolve().parent.parent / "config.toml"
    text = config_path.read_text()
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("url"):
            m = re.match(r'url\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    print("ERROR: database url not found in config.toml", file=sys.stderr)
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description="Backfill pricing_policy and weighted_tokens")
    parser.add_argument("--dry-run", action="store_true", help="Print planned updates without writing")
    parser.add_argument("--batch-size", type=int, default=500, help="Rows per UPDATE batch")
    args = parser.parse_args()

    db_url = load_db_url()
    conn = psycopg2.connect(db_url)
    conn.autocommit = False
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)

    # Count rows needing backfill
    cur.execute("SELECT COUNT(*) AS cnt FROM usage_records WHERE pricing_policy IS NULL")
    total = cur.fetchone()["cnt"]
    print(f"Rows to backfill: {total}")
    if total == 0:
        print("Nothing to do.")
        conn.close()
        return

    # Preload all pricing policies
    cur.execute("SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies")
    policies = {row["id"]: dict(row) for row in cur.fetchall()}

    # Build model_name -> pricing_policy mapping
    cur.execute("""
        SELECT m.name AS model_name, m.pricing_policy_id
        FROM models m
        WHERE m.pricing_policy_id IS NOT NULL
    """)
    model_policy_map = {}
    for row in cur.fetchall():
        policy = policies.get(row["pricing_policy_id"])
        if policy:
            model_policy_map[row["model_name"]] = policy

    print(f"Pricing policies loaded: {len(policies)}")
    print(f"Models with policy: {len(model_policy_map)}")

    # Process in batches
    updated = 0
    skipped = 0
    batch_num = 0

    cur.execute("""
        SELECT id, model_name, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens
        FROM usage_records
        WHERE pricing_policy IS NULL
        ORDER BY created_at
    """)
    rows = cur.fetchall()

    for i in range(0, len(rows), args.batch_size):
        batch = rows[i : i + args.batch_size]
        batch_num += 1
        updates = []

        for row in batch:
            policy = model_policy_map.get(row["model_name"])
            if policy is None:
                skipped += 1
                continue

            inp = row["input_tokens"] or 0
            out = row["output_tokens"] or 0
            cr = row["cache_read_tokens"] or 0
            cc = row["cache_creation_tokens"] or 0
            wt = calculate_weighted_tokens(policy, inp, out, cr, cc)

            updates.append((json.dumps(policy, default=str), wt, row["id"]))
            updated += 1

        if not updates:
            continue

        if args.dry_run:
            for policy_json, wt, rid in updates[:3]:
                print(f"  {rid}: weighted_tokens={wt}")
            if len(updates) > 3:
                print(f"  ... and {len(updates) - 3} more")
        else:
            psycopg2.extras.execute_batch(
                cur,
                "UPDATE usage_records SET pricing_policy = %s::jsonb, weighted_tokens = %s WHERE id = %s",
                updates,
            )
            conn.commit()

        print(f"Batch {batch_num}: {len(updates)} rows {'(dry-run)' if args.dry_run else 'updated'}")

    cur.close()
    conn.close()
    print(f"\nDone. Updated: {updated}, Skipped (no policy): {skipped}, Total: {total}")


if __name__ == "__main__":
    main()
