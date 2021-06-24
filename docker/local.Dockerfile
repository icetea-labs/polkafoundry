FROM debian:bullseye-slim
LABEL description="Binary for Polkafoundry Collator"

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkafoundry polkafoundry && \
	mkdir -p /polkafoundry/.local/share && \
	mkdir /data && \
	chown -R polkafoundry:polkafoundry /data && \
	ln -s /data /polkafoundry/.local/share/polkafoundry && \
	rm -rf /usr/bin /usr/sbin

USER polkafoundry

COPY --chown=polkafoundry bin/polkafoundry /polkafoundry/polkafoundry

RUN chmod uog+x /polkafoundry/polkafoundry

EXPOSE 30333 9933 9944

VOLUME ["/data"]

ENTRYPOINT ["/polkafoundry/polkafoundry"]
