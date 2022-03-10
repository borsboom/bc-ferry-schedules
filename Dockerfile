FROM rust:1.59-bullseye as builder
WORKDIR /build
COPY . .
RUN cargo install --path scraper

FROM debian:bullseye-slim as scraper
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/ferrysched_scraper /usr/local/bin/ferrysched_scraper
ENTRYPOINT ["ferrysched_scraper"]
