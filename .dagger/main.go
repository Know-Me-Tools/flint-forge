// Flint Forge CI — Dagger module.
//
// Bootstrap (requires the dagger CLI, not present at scaffold time):
//
//	curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
//	dagger develop                 # generates ./internal/dagger bindings + go.mod
//	dagger call check --source=.   # runs fmt + clippy::pedantic(-D warnings) + cargo check
//
// The Check function wraps scripts/ci-check.sh in a pinned Rust container so local and CI
// runs are byte-identical. The underlying checks are proven green locally (see p0-c001 gate).
package main

import (
	"context"

	"dagger/flint-forge-ci/internal/dagger"
)

type FlintForgeCi struct{}

// Check runs the canonical gate inside a pinned Rust toolchain container.
func (m *FlintForgeCi) Check(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.base(source).
		WithExec([]string{"bash", "scripts/ci-check.sh"}).
		Stdout(ctx)
}

// CheckDb runs the DB-integration test stage against an ephemeral Postgres service
// built from docker/postgres (PG18 + pgvector + pg_graphql). It binds the service,
// exports DATABASE_URL, and runs scripts/ci-test.sh (unit + db-integration).
//
// NOTE (p35-c003): authored but NOT executed in the change environment — the `dagger`
// CLI is not available there. The service-binding shape follows the standard Dagger
// pattern; run `dagger call check-db --source=.` on a host with the CLI to verify.
func (m *FlintForgeCi) CheckDb(ctx context.Context, source *dagger.Directory) (string, error) {
	// Build the pinned Postgres image from the repo Dockerfile and expose 5432.
	pg := dag.Container().
		Build(source.Directory("docker/postgres")).
		WithEnvVariable("POSTGRES_PASSWORD", "postgres").
		WithEnvVariable("POSTGRES_DB", "flint").
		WithExposedPort(5432).
		AsService()

	return m.base(source).
		WithServiceBinding("postgres", pg).
		// 12-factor: config from env. The service is reachable at host "postgres".
		WithEnvVariable("DATABASE_URL", "postgres://postgres:postgres@postgres:5432/flint").
		WithExec([]string{"bash", "scripts/ci-test.sh"}).
		Stdout(ctx)
}

// base builds the toolchain container with a warm cargo registry cache.
func (m *FlintForgeCi) base(source *dagger.Directory) *dagger.Container {
	registry := dag.CacheVolume("flint-forge-cargo-registry")
	target := dag.CacheVolume("flint-forge-target")
	return dag.Container().
		From("rust:1.90-bookworm").
		WithMountedCache("/usr/local/cargo/registry", registry).
		WithMountedDirectory("/src", source).
		WithMountedCache("/src/target", target).
		WithWorkdir("/src").
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"})
}
