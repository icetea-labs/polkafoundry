FROM paritytech/ci-linux:production as builder
LABEL description="This is the build stage for PolkaFoundry. Here we create the binary."

ARG PROFILE=release
WORKDIR /polkafoundry

COPY . /polkafoundry

RUN cargo build --$PROFILE

# ===== Build image state ======
FROM debian:bullseye-slim
LABEL description="Binary for Polkafoundry Collator"

WORKDIR /polkafoundry
ARG PROFILE=release

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkafoundry polkafoundry && \
	mkdir -p /polkafoundry/.local/share && \
	mkdir /data && \
	chown -R polkafoundry:polkafoundry /data && \
	ln -s /data /polkafoundry/.local/share/polkafoundry && \
	rm -rf /usr/bin /usr/sbin

USER polkafoundry

COPY --chown=polkafoundry --from=builder /polkafoundry/target/$PROFILE/polkafoundry /polkafoundry/polkafoundry

RUN chmod a+x /polkafoundry/polkafoundry

EXPOSE 30333 9933 9944

VOLUME ["/data"]

ENTRYPOINT ["/polkafoundry/polkafoundry"]
