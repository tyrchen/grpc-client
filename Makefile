build:
	@cargo build

ui:
	@cd ui && yarn dev

test:
	@cargo nextest run --all-features

ui-test:
	@cd ui && yarn test

release:
	@cargo release tag --execute
	@git cliff -o CHANGELOG.md
	@git commit -a -n -m "Update CHANGELOG.md" || true
	@git push origin master
	@cargo release push --execute

update-submodule:
	@git submodule update --init --recursive --remote

.PHONY: build test release update-submodule ui
