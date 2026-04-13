FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata libgcc && \
    addgroup -S gateway && adduser -S gateway -G gateway

WORKDIR /app

COPY llm-gateway .
COPY config.toml .

RUN mkdir -p /app/data && chown -R gateway:gateway /app

USER gateway

EXPOSE 8080

VOLUME ["/app/data"]

CMD ["./llm-gateway"]
