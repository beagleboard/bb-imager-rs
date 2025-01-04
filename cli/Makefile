build-cli-%:
	$(info "Building CLI for $*")
	$(RUST_BUILDER) build --release --target $* -p bb-imager-cli

# Special rule for macos universal binary
build-cli-universal-apple-darwin: build-cli-aarch64-apple-darwin build-cli-x86_64-apple-darwin
	$(info "Building CLI for universal-apple-darwin")
	mkdir -p target/universal-apple-darwin/release
	lipo -create target/x86_64-apple-darwin/release/bb-imager-cli target/aarch64-apple-darwin/release/bb-imager-cli -output target/universal-apple-darwin/release/bb-imager-cli

package-cli-linux-xz-%: build-cli-% manpage
	$(info "Create CLI xz $*")
	mkdir -p ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir
	cp target/$*/release/bb-imager-cli ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	cp -r target/man ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	tar cfJ ${RELEASE_DIR_LINUX}/$*/bb-imager-cli.xz ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir/
	rm -rf ${RELEASE_DIR_LINUX}/$*/bb-imager-cli-tempdir

package-cli-linux-deb-%: build-cli-% manpage
	$(info "Create CLI Linux deb $*")
	cargo deb --target $* -p bb-imager-cli --no-build -o ${RELEASE_DIR_LINUX}/$*/bb-imager-cli.deb --no-strip

package-cli-darwin-zip-%: build-cli-%
	$(info "Create CLI Darwin zip $*")
	mkdir -p ${RELEASE_DIR_DARWIN}/$*
	zip -j ${RELEASE_DIR_DARWIN}/$*/bb-imager-cli.zip target/$*/release/bb-imager-cli

package-cli-windows-zip-%: build-cli-%
	$(info "Create CLI Windows zip $*")
	mkdir -p ${RELEASE_DIR_WINDOWS}/$*
	zip -j ${RELEASE_DIR_WINDOWS}/$*/bb-imager-cli.zip target/$*/release/bb-imager-cli.exe

manpage:
	$(info "Generate CLI Manpages")
	mkdir -p target/man
	cargo xtask cli-man target/man
	gzip target/man/* -f
