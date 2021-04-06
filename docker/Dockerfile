FROM paritytech/ci-linux:production as builder
LABEL description="This is the build stage for Polkafoundry. Here we create the binary."

ARG PROFILE=release
WORKDIR /polkafoundry

COPY . /polkafoundry

RUN nice -n 20 cargo build --$PROFILE

# ===== SECOND STAGE ======

FROM debian:bullseye-slim
LABEL description="Binary for Polkafoundry Collator"

ARG PROFILE=release
#COPY --from=builder /polkafoundry/target/$PROFILE/polkafoundry /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkafoundry polkafoundry && \
	mkdir -p /polkafoundry/.local/share && \
	mkdir /data && \
	chown -R polkafoundry:polkafoundry /data && \
	ln -s /data /polkafoundry/.local/share/polkafoundry && \
	rm -rf /usr/bin /usr/sbin

USER polkafoundry

#COPY --chown=polkafoundry bin/polkafoundry /polkafoundry/polkafoundry
COPY --from=builder /polkafoundry/target/$PROFILE/polkafoundry /polkafoundry/polkafoundry

RUN chmod a+x /polkafoundry/polkafoundry

EXPOSE 30333 9933 9944

VOLUME ["/data"]

ENTRYPOINT ["/polkafoundry/polkafoundry"]
