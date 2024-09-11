FROM rust:1.76 as planner
WORKDIR /app
RUN cargo install cargo-chef --locked
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.76 as builder
WORKDIR /app
RUN cargo install cargo-chef --locked
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

# Runtime image
FROM cgr.dev/chainguard/glibc-dynamic:latest-dev
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /app/target/release/mrrper /app/mrrper

# Run the app
CMD ./mrrper
