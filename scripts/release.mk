# Helpers for creating Gitlab Releases
CURL ?= $(shell which curl)

PACKAGE_REGISTRY_URL = ${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic
PACKAGE_REGISTRY_GUI_URL = ${PACKAGE_REGISTRY_URL}/bb-imager-gui/${VERSION}
PACKAGE_REGISTRY_CLI_URL = ${PACKAGE_REGISTRY_URL}/bb-imager-cli/${VERSION}

# Upload to Package Registry. Also upload checksum
#
# Arg 1: source
# Arg 2: destination
define upload_artifact
	sha256sum $(1) > $(1).sha256
	${CURL} --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file $(1) $(2)
	${CURL} --fail-with-body --header "JOB-TOKEN: ${CI_JOB_TOKEN}" --upload-file $(1).sha256 $(2).sha256
endef

upload-artifact-linux-%:
	$(info "Upload Linux $* artifacts")
	$(call upload_artifact,"${RELEASE_DIR_LINUX}/$*/BeagleBoardImager.AppImage","${PACKAGE_REGISTRY_GUI_URL}/$*/bb-imager-gui.AppImage")
	$(call upload_artifact,"${RELEASE_DIR_LINUX}/$*/bb-imager-gui.deb","${PACKAGE_REGISTRY_GUI_URL}/$*/bb-imager-gui.deb")
	$(call upload_artifact,"${RELEASE_DIR_LINUX}/$*/bb-imager-cli.xz","${PACKAGE_REGISTRY_CLI_URL}/$*/bb-imager-cli.xz")
	$(call upload_artifact,"${RELEASE_DIR_LINUX}/$*/bb-imager-cli.deb","${PACKAGE_REGISTRY_CLI_URL}/$*/bb-imager-cli.deb")

upload-artifact-darwin-%:
	$(info "Upload Darwin $* artifacts")
	$(call upload_artifact,"${RELEASE_DIR_DARWIN}/$*/BeagleBoardImager.dmg","${PACKAGE_REGISTRY_GUI_URL}/$*/bb-imager-gui.dmg")
	$(call upload_artifact,"${RELEASE_DIR_DARWIN}/$*/bb-imager-cli.zip","${PACKAGE_REGISTRY_CLI_URL}/$*/bb-imager-cli.zip")

upload-artifact-windows-%:
	$(info "Upload Windows $* artifacts")
	$(call upload_artifact,"${RELEASE_DIR_WINDOWS}/$*/bb-imager-gui.zip","${PACKAGE_REGISTRY_GUI_URL}/$*/bb-imager-gui.zip")
	$(call upload_artifact,"${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli.zip","${PACKAGE_REGISTRY_CLI_URL}/$*/bb-imager-cli.zip")

upload-artifact-linux: upload-artifact-linux-x86_64-unknown-linux-gnu upload-artifact-linux-aarch64-unknown-linux-gnu upload-artifact-linux-armv7-unknown-linux-gnueabihf;

upload-artifact-darwin: upload-artifact-darwin-x86_64-apple-darwin upload-artifact-darwin-aarch64-apple-darwin upload-artifact-darwin-universal-apple-darwin;

upload-artifact-windows: upload-artifact-windows-x86_64-pc-windows-gnu;

release-notes:
	$(info "Generate release notes for $VERSION")
	curl -H "PRIVATE-TOKEN: ${CI_API_TOKEN}" "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/repository/changelog?version=${VERSION}" | jq -r .notes > release_notes.md
