FROM rust:alpine AS builder
LABEL maintainer="mingcheng <mingcheng@apache.org>"

# Set the working directory
ENV BUILD_DIR=/build

# Add necessary build dependencies
RUN apk add --no-cache build-base git musl-dev libressl-dev pkgconfig perl

# Update the latest stable version of rust toolkit
RUN rustup default stable && rustup override set stable

# Start building the application
COPY . ${BUILD_DIR}
WORKDIR ${BUILD_DIR}

# Build the application
RUN cargo update \
    && cargo build --release \
    && cp target/release/aigitcommit /bin/aigitcommit

# Stage2
FROM alpine

# # Install timezone data and set timezone
ENV TZ="Asia/Shanghai"
RUN apk update \
    && apk add --no-cache tzdata git curl \
    && ln -snf /usr/share/zoneinfo/$TZ /etc/localtime \
    && echo $TZ > /etc/timezone

# # Copy the binary from the builder stage
COPY --from=builder /bin/aigitcommit /bin/aigitcommit

# # Set the working directory
WORKDIR /repo

# # Define the command to run the application
ENTRYPOINT ["/bin/aigitcommit"]
