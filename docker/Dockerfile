FROM paritytech/ci-linux:production as builder
LABEL description="This is the build stage for NewPolka. Here we create the binary."

ARG PROFILE=release
WORKDIR /newpolka

COPY . /newpolka
# NOTE: Modify number jobs depend on CPUs. This avoid crash when linker pokadot-service
RUN cargo build -j3 --$PROFILE

# ===== SECOND STAGE ======

FROM debian:buster-slim
LABEL description="This is the 2nd stage: a very small image where we copy the newpolka binary."
ARG PROFILE=release
COPY --from=builder /newpolka/target/$PROFILE/polkafoundry /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /newpolka newpolka && \
	mkdir -p /newpolka/.local/share && \
	mkdir /data && \
	chown -R newpolka:newpolka /data && \
	ln -s /data /newpolka/.local/share/newpolka && \
	rm -rf /usr/bin /usr/sbin

USER newpolka
EXPOSE 40333 9844
VOLUME ["/data"]

CMD ["/usr/local/bin/polkafoundry"]
