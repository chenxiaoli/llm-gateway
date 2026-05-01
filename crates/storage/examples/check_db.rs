fn main() {
    let db_path = std::path::Path::new("/workspace/llm-gateway/gateway.db");
    if !db_path.exists() {
        eprintln!("DB not found");
        return;
    }
    let conn = rusqlite::Connection::open(db_path).unwrap();

    println!("=== MODELS ===");
    let mut stmt = conn.prepare("SELECT id, name, pricing_policy_id FROM models").unwrap();
    let models: Vec<(String, String, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    for (id, name, pid) in &models {
        println!("{}: {} -> policy={:?}", id, name, pid);
    }

    println!("\n=== CHANNEL_MODELS ===");
    let mut stmt = conn.prepare("SELECT id, model_id, pricing_policy_id, markup_ratio FROM channel_models").unwrap();
    let cms: Vec<(String, String, Option<String>, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    for (id, mid, pid, mr) in &cms {
        let mname = models.iter().find(|(x, _, _)| x == mid).map(|(_, n, _)| n.as_str()).unwrap_or("?");
        println!("{}: model={} ({}) policy={:?} markup={}", id, mid, mname, pid, mr);
    }

    println!("\n=== PRICING POLICIES ===");
    let mut stmt = conn.prepare("SELECT id, name, billing_type, config FROM pricing_policies").unwrap();
    let policies: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    for (id, name, bt, config) in &policies {
        println!("{}: {} [{}] config={}", id, name, bt, &config[..config.len().min(120)]);
    }
}
