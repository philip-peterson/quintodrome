package main

import (
	"context"

	"dagger/quintodrome/internal/dagger"
)

type Quintodrome struct{}

// Ci runs the full Linux CI: lint, build, and test for both Go and JS.
func (m *Quintodrome) Ci(ctx context.Context,
	// +defaultPath="."
	src *dagger.Directory,
) error {
	if err := m.LintGo(ctx, src); err != nil {
		return err
	}
	if err := m.FmtGo(ctx, src); err != nil {
		return err
	}
	if err := m.BuildGo(ctx, src); err != nil {
		return err
	}
	if err := m.TestGo(ctx, src); err != nil {
		return err
	}
	if err := m.LintJS(ctx, src); err != nil {
		return err
	}
	if err := m.BuildJS(ctx, src); err != nil {
		return err
	}
	if err := m.TestJS(ctx, src); err != nil {
		return err
	}
	return nil
}

// LintGo runs golangci-lint with the repo's .golangci.yml.
func (m *Quintodrome) LintGo(ctx context.Context, src *dagger.Directory) error {
	_, err := goContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo").
		WithExec([]string{"sh", "-c", "curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh | sh -s -- -b /usr/local/bin v2.12.0"}).
		WithExec([]string{"golangci-lint", "run", "--timeout", "2m"}).
		Sync(ctx)
	return err
}

// FmtGo checks that goimports and go mod tidy leave the tree clean.
func (m *Quintodrome) FmtGo(ctx context.Context, src *dagger.Directory) error {
	_, err := goContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo").
		WithExec([]string{"sh", "-c", `
go run golang.org/x/tools/cmd/goimports@latest -w $(find . -name '*.go' | grep -v '_gen.go$' | grep -v '.pb.go$')
go mod tidy
test -z "$(git status --porcelain)"
`}).
		Sync(ctx)
	return err
}

// BuildGo compiles the Navidrome server and all packages.
func (m *Quintodrome) BuildGo(ctx context.Context, src *dagger.Directory) error {
	_, err := goContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo").
		WithExec([]string{"go", "build", "-tags", "netgo,sqlite_fts5", "./..."}).
		Sync(ctx)
	return err
}

// TestGo runs the Go test suite.
func (m *Quintodrome) TestGo(ctx context.Context, src *dagger.Directory) error {
	_, err := goContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo").
		WithExec([]string{"go", "test", "-tags", "netgo,sqlite_fts5", "./..."}).
		Sync(ctx)
	return err
}

// LintJS runs prettier and eslint in the web UI.
func (m *Quintodrome) LintJS(ctx context.Context, src *dagger.Directory) error {
	_, err := jsContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo/ui").
		WithExec([]string{"npm", "ci", "--ignore-scripts"}).
		WithExec([]string{"npm", "run", "check-formatting"}).
		WithExec([]string{"npm", "run", "lint"}).
		Sync(ctx)
	return err
}

// BuildJS builds the Navidrome web UI.
func (m *Quintodrome) BuildJS(ctx context.Context, src *dagger.Directory) error {
	_, err := jsContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo/ui").
		WithExec([]string{"npm", "ci", "--ignore-scripts"}).
		WithExec([]string{"npm", "run", "build"}).
		Sync(ctx)
	return err
}

// TestJS runs the JS test suite.
func (m *Quintodrome) TestJS(ctx context.Context, src *dagger.Directory) error {
	_, err := jsContainer().
		WithDirectory("/repo", src).
		WithWorkdir("/repo/ui").
		WithExec([]string{"npm", "ci", "--ignore-scripts"}).
		WithExec([]string{"npm", "test"}).
		Sync(ctx)
	return err
}

func goContainer() *dagger.Container {
	return dag.Container().From("golang:1.26")
}

func jsContainer() *dagger.Container {
	return dag.Container().From("node:24")
}
