FROM debian:bullseye-slim
LABEL description="Binary for Polkadot Relaychain"

RUN useradd -m -u 1000 -U -s /bin/sh -d /polkadot polkadot && \
	mkdir -p /polkadot/.local/share && \
	mkdir /data && \
	chown -R polkadot:polkadot /data && \
	ln -s /data /polkadot/.local/share/polkadot && \
	rm -rf /usr/bin /usr/sbin

USER polkadot

COPY --chown=polkadot bin/polkadot /polkadot/polkadot

RUN chmod uog+x /polkadot/polkadot

EXPOSE 30333 9933 9944

VOLUME ["/data"]

ENTRYPOINT ["/polkadot/polkadot"]
