FROM rust:1.61 as build

RUN USER=root cargo new --bin qr-bill-bot
WORKDIR /qr-bill-bot

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm -Rf src

COPY ./src ./src

RUN rm ./target/release/deps/qr_bill_bot*
RUN cargo build --release

FROM rust:1.61-slim-buster

COPY --from=build /qr-bill-bot/target/release/qr-bill-bot .

CMD ["./qr-bill-bot"]
