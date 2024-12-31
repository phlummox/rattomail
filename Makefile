
all:
	cargo build --release

dev:
	cargo build --features test_env_executables

install:
	strip target/release/rattomail
	install -m 755 target/release/rattomail /usr/sbin/sendmail
	chmod u+s /usr/sbin/sendmail

test:
	cargo test

bogus_test:
	echo | cargo run --features="test_env_executables" --bin bogus_rattomail -- -X /dev/stderr -f ppp ooo

##
# statically-linked binaries

static: static_binaries/bogus_rattomail static_binaries/rattomail

static_binaries/bogus_rattomail static_binaries/rattomail:
	mkdir -p static_binaries
	docker -D run --rm -i -v $(PWD):/work --workdir /work rust:1.83.0-alpine3.21 \
		sh -c 'set -x && \
						apk --update add musl-dev && \
						cargo clean && \
						cargo build --release --features test_env_executables && \
						cp target/release/bogus_rattomail static_binaries && \
						cp target/release/rattomail static_binaries && \
						chown -R nobody:nobody static_binaries && \
						chmod o+rwx static_binaries && \
						cargo clean'

DEBFILE_NAME := $(shell ./print-deb-name.pl || kill $$PPID )

deb: $(DEBFILE_NAME)

$(DEBFILE_NAME): static_binaries/rattomail
	./build-deb.pl $<

docker-test: $(DEBFILE_NAME)
	prove ./docker-test.pl :: $<

VERSION := $(shell set -e; ./print-deb-name.pl --ver-arch | awk '{ print $$1; }' || kill $$PPID )
ARCHITECTURE := $(shell set -e; ./print-deb-name.pl --ver-arch | awk '{ print $$2; }' || kill $$PPID )
TGZ_FILE = rattomail-$(VERSION)-linux-$(ARCHITECTURE).tgz

tgz: $(TGZ_FILE)

$(TGZ_FILE): static_binaries/rattomail
	tempdir=`mktemp -d` && \
	cp $< $$tempdir && \
	strip $$tempdir/rattomail && \
	fakeroot tar -C $$tempdir -cf $@ rattomail

clean:
	cargo clean
	-rm -rf *.deb static_binaries *.tgz

.PHONY: all test docker-test deb clean static install tgz

.DELETE_ON_ERROR:

