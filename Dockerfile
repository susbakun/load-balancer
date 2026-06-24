FROM rust:1.89

WORKDIR /app

COPY . .

RUN cargo build

EXPOSE 8085

CMD [ "cargo", "run" ]
