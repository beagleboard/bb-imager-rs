RUST_CLI_FEATURE_ARGS = $(RUST_FEATURE_ARGS)

# Enable CLI specific features
ifdef PB2_MSPM0
	RUST_CLI_FEATURE_ARGS += -F pb2_mspm0
endif

build-cli-%:
	$(info "Building CLI for $*")
	$(RUST_BUILDER) build --release --target $* -p bb-imager-cli $(RUST_CLI_FEATURE_ARGS)

# Special rule for macos universal binary
build-cli-universal-apple-darwin: build-cli-aarch64-apple-darwin build-cli-x86_64-apple-darwin
	$(info "Building CLI for universal-apple-darwin")
	mkdir -p target/universal-apple-darwin/release
	lipo -create target/x86_64-apple-darwin/release/bb-imager-cli target/aarch64-apple-darwin/release/bb-imager-cli -output target/universal-apple-darwin/release/bb-imager-cli

package-cli-linux-xz-%: build-cli-% manpage shell-completion-bash shell-completion-zsh
	$(info "Create CLI xz $*")
	mkdir -p ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir
	cp target/$*/release/bb-imager-cli ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	cp -r target/man ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	cp -r target/shell-completion ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	tar cfJ ${RELEASE_DIR_LINUX}/$*/bb-imager-cli.tar.xz ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	rm -rf ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir

package-cli-linux-deb-%: build-cli-% manpage shell-completion-bash shell-completion-zsh
	$(info "Create CLI Linux deb $*")
ifeq ($(RUST_BUILDER_NAME), cross)
	$(CROSS_UTIL) run --target $* -- "cargo deb --target $* -p bb-imager-cli --no-build -o ${RELEASE_DIR_LINUX}/$*/bb-imager-cli.deb --no-strip"
else
	$(RUST_BUILDER) deb --target $* -p bb-imager-cli --no-build -o ${RELEASE_DIR_LINUX}/$*/bb-imager-cli.deb --no-strip
endif

package-cli-darwin-zip-%: build-cli-% manpage shell-completion-zsh
	$(info "Create CLI Darwin zip $*")
	mkdir -p ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli-tempdir
	cp -r target/man ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli-tempdir/
	cp -r target/shell-completion ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli-tempdir/
	cp target/$*/release/bb-imager-cli ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli-tempdir/
	# Required to get the desired directory structure
	cd ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli-tempdir && zip -r ../bb-imager-cli.zip *
	rm -rf ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli-tempdir/

package-cli-windows-zip-%: build-cli-% shell-completion-powershell
	$(info "Create CLI Windows zip $*")
	mkdir -p ${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli-tempdir
	cp target/$*/release/bb-imager-cli.exe ${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli-tempdir/
	cp -r target/shell-completion ${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli-tempdir/
	# Required to get the desired directory structure
	cd ${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli-tempdir/ && zip -r ../bb-imager-cli.zip *
	rm -rf ${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli-tempdir

manpage:
	$(info "Generate CLI Manpages")
	rm -rf target/man
	mkdir -p target/man
	$(CARGO_PATH) xtask $(RUST_CLI_FEATURE_ARGS) cli-man target/man
	gzip target/man/*

shell-completion-%:
	$(info "Generate CLI Shell Completion")
	mkdir -p target/shell-completion
	$(CARGO_PATH) xtask $(RUST_CLI_FEATURE_ARGS) cli-shell-complete $* target/shell-completion
