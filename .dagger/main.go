package main

import (
	"context"

	"dagger/quintodrome/internal/dagger"
)

type Quintodrome struct{}

// Ci runs the full Linux CI: Go build + test and JS build + test.
func (m *Quintodrome) Ci(ctx context.Context,
	// +defaultPath="."
	src *dagger.Directory,
) error {
	if err := m.BuildGo(ctx, src); err != nil {
		return err
	}
	if err := m.TestGo(ctx, src); err != nil {
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
