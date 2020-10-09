FROM rust:latest AS build

WORKDIR /caolo

# ============= cache dependencies =============
WORKDIR /caolo
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN mkdir src/
RUN echo "fn main() {}" > ./src/dummy.rs
RUN sed -i 's/src\/main.rs/src\/dummy.rs/' Cargo.toml
RUN cargo build --release

WORKDIR /caolo
COPY ./ ./
RUN cargo install --path . --root .

# ---------- Copy the built binary to a scratch container, to minimize the image size ----------

FROM ubuntu:20.04
WORKDIR /caolo

COPY --from=build /caolo/bin/ ./

RUN ls -al /caolo

ENTRYPOINT ["./cao-profile-collector"]
