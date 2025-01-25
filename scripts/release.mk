# Helpers for creating Gitlab Releases
CURL ?= $(shell which curl)

release-notes:
	$(info "Generate release notes for $VERSION")
	curl -H "PRIVATE-TOKEN: ${CI_API_TOKEN}" "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/repository/changelog?version=${VERSION}" | jq -r .notes > release_notes.md
