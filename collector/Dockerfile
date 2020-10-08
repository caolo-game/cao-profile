FROM rust:latest AS build

RUN apt-get update
RUN apt-get install sudo postgresql postgresql-contrib -y
RUN service postgresql start

WORKDIR /caolo
RUN cargo install diesel_cli --root . --no-default-features --features="postgres"

ENV DATABASE_URL=postgres://postgres:postgres@localhost:5432/caolo

# ============= cache dependencies =============
WORKDIR /caolo
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN mkdir src/
RUN echo "fn main() {}" > ./src/dummy.rs
RUN sed -i 's/src\/main.rs/src\/dummy.rs/' Cargo.toml
RUN cargo build --release

WORKDIR /caolo
COPY ./build.sh ./
COPY ./ ./
COPY ./migrations/ ./migrations/

WORKDIR /caolo
RUN bash ./build.sh

# ---------- Copy the built binary to a scratch container, to minimize the image size ----------

FROM ubuntu:20.04
WORKDIR /caolo
RUN apt-get update
RUN apt-get install curl libpq-dev -y

COPY ./migrations/ ./migrations/
COPY ./release.sh ./release.sh
COPY --from=build /caolo/bin/ ./

RUN ls -al /caolo

ENTRYPOINT ["./cao-profile-collector"]
