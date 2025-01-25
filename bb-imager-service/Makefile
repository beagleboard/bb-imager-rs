PACKAGE_NAME_SERVICE = bb-imager-service
RELEASE_DIR_SERVICE = ${RELEASE_DIR}/${PACKAGE_NAME_SERVICE}/${PACKAGE_VERSION}

build-service-%:
	$(info "Building D-Bus Service for $*")
	$(RUST_BUILDER) build --release --target $* -p bb-imager-service

package-service-linux-deb-%: build-service-%
	$(info "Create D-Bus Service Linux deb $*")
ifeq ($(RUST_BUILDER_NAME), cross)
	$(CROSS_UTIL) run --target $* -- "cargo deb --target $* -p bb-imager-service --no-build -o ${RELEASE_DIR_SERVICE}/${PACKAGE_NAME_SERVICE}-$*.deb --no-strip"
else
	$(RUST_BUILDER) deb --target $* -p bb-imager-service --no-build -o ${RELEASE_DIR_SERVICE}/${PACKAGE_NAME_SERVICE}-$*.deb --no-strip
endif

package-service-linux-xz-%: build-service-%
	$(info "Create D-Bus Service Linux xz $*")
	install -D target/$*/release/bb-imager-service -t ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir/usr/bin/
	install -D -m 644 $(SERVICE_ASSETS)/dbus-org.beagleboard.ImagingService.service -t ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir/usr/lib/systemd/system/
	install -D -m 644 $(SERVICE_ASSETS)/org.beagleboard.ImagingService.conf -t ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir/usr/share/dbus-1/system.d/
	install -D -m 644 $(SERVICE_ASSETS)/org.beagleboard.ImagingService.policy -t ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir/usr/share/polkit-1/actions/
	install -D -m 644 $(SERVICE_ASSETS)/org.beagleboard.ImagingService.service -t ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir/usr/share/dbus-1/system-services/
	cd ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir/ && tar cfJ ${RELEASE_DIR_SERVICE}/${PACKAGE_NAME_SERVICE}-$*.tar.xz *
	rm -rf ${RELEASE_DIR_SERVICE}/bb-imager-service-tempdir
