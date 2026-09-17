.PHONY: srpm

VERSION = $(shell grep '^Version:' $(spec) | awk '{print $$2}')
NAME = $(shell grep '^Name:' $(spec) | awk '{print $$2}')
CRATE   = $(NAME:rust-%=%)


srpm:
	dnf -y install cargo-vendor-filterer cargo rust
	curl -Lo $(CRATE)-$(VERSION).crate "https://crates.io/api/v1/crates/$(CRATE)/$(VERSION)/download"
	tar xf $(CRATE)-$(VERSION).crate
	cd $(CRATE)-$(VERSION) && cargo-vendor-filterer vendor > ../cargo-config.toml
	tar cJf $(CRATE)-$(VERSION)-vendor.tar.xz -C $(CRATE)-$(VERSION) vendor
	rpmbuild -bs $(spec) \
		--define "_sourcedir $(CURDIR)" \
		--define "_srcrpmdir $(outdir)"
