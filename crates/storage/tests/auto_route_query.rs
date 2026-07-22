use llm_gateway_storage::{postgres::PostgresStorage, Model, Storage};

#[sqlx::test(migrator = "llm_gateway_storage::MIGRATOR")]
async fn list_models_with_capabilities_filters_correctly(pool: sqlx::PgPool) {
    let storage = PostgresStorage::from_pool(pool);

    // Seed three models in org_default: text-only, vision-capable, full-capable.
    let now = chrono::Utc::now();
    for (name, vision, tools) in [
        ("text-only", false, false),
        ("vision-model", true, false),
        ("full-capable", true, true),
    ] {
        let m = Model {
            id: name.to_string(),
            owner_org_id: Some("org_default".to_string()),
            name: name.to_string(),
            model_type: None,
            pricing_policy_id: None,
            supports_vision: vision,
            supports_tools: tools,
            created_at: now,
        };
        storage.create_model("org_default", &m).await.unwrap();
    }

    let candidates: Vec<String> = vec![
        "text-only".into(),
        "vision-model".into(),
        "full-capable".into(),
    ];

    // No capabilities required -> all 3 eligible.
    let all = storage
        .list_models_with_capabilities("org_default", false, false, &candidates)
        .await
        .unwrap();
    assert_eq!(all.len(), 3, "no caps required: all 3 should match");

    // Vision required -> 2 eligible (vision-model + full-capable).
    let vis = storage
        .list_models_with_capabilities("org_default", true, false, &candidates)
        .await
        .unwrap();
    assert_eq!(vis.len(), 2);
    assert!(vis.iter().all(|m| m.supports_vision));

    // Both required -> 1 eligible (full-capable only).
    let both = storage
        .list_models_with_capabilities("org_default", true, true, &candidates)
        .await
        .unwrap();
    assert_eq!(both.len(), 1);
    assert_eq!(both[0].name, "full-capable");

    // Candidate pool restriction honored.
    let small_pool = storage
        .list_models_with_capabilities(
            "org_default",
            false,
            false,
            &["text-only".to_string()],
        )
        .await
        .unwrap();
    assert_eq!(small_pool.len(), 1);
    assert_eq!(small_pool[0].name, "text-only");
}
