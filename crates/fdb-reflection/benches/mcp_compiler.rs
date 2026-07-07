use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fdb_reflection::compilers::mcp::McpCompiler;
use fdb_reflection::model::{Column, DatabaseModel, Table};

fn make_model(table_count: usize) -> DatabaseModel {
    let tables = (0..table_count)
        .map(|i| Table {
            schema: "public".into(),
            name: format!("table_{i}"),
            columns: vec![
                Column {
                    name: "id".into(),
                    pg_type: "uuid".into(),
                    nullable: false,
                    default: Some("gen_random_uuid()".into()),
                },
                Column {
                    name: "name".into(),
                    pg_type: "text".into(),
                    nullable: false,
                    default: None,
                },
                Column {
                    name: "created_at".into(),
                    pg_type: "timestamptz".into(),
                    nullable: false,
                    default: Some("now()".into()),
                },
            ],
            pk: vec!["id".into()],
            fk: vec![],
            rls_enabled: true,
            vault_key: None,
        })
        .collect();
    DatabaseModel {
        tables,
        functions: vec![],
        views: vec![],
        version: 1,
    }
}

fn bench_mcp_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("McpCompiler::compile");
    for size in [10, 25, 50, 100] {
        let model = make_model(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &model, |b, m| {
            b.iter(|| McpCompiler::compile(black_box(m)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_mcp_compile);
criterion_main!(benches);
