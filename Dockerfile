FROM debian:stable-slim

WORKDIR /opt/app

RUN apt-get update -y && apt-get install -y ca-certificates
RUN dpkg-reconfigure -p critical ca-certificates

COPY ./.env ./.env
COPY ./resources ./resources
COPY ./target/server/release/hoverboard_server ./dist/hoverboard_server

CMD ["/opt/app/dist/hoverboard_server"]
