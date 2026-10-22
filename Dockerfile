# Stage 1: Builder
FROM rust:alpine3.23 AS builder
LABEL maintainer="mingcheng <mingcheng@apache.org>"

# Install build dependencies required for compilation
RUN apk add --no-cache \
    build-base \
    git \
    musl-dev \
    libressl-dev \
    pkgconfig \
    perl

WORKDIR /build

COPY . .

# Build the application with optimizations
RUN cargo build --release --locked && \
    strip target/release/aigitcommit && \
    cp target/release/aigitcommit /bin/aigitcommit

# Stage 2: Runtime
FROM alpine:3 AS runtime

# Set timezone (configurable via build args)
ARG TZ=Asia/Shanghai
ENV TZ=${TZ}

# Install only runtime dependencies
RUN apk add --no-cache \
    tzdata \
    git \
    curl \
    ca-certificates && \
    ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && \
    echo $TZ > /etc/timezone && \
    # Clean up apk cache to reduce image size
    rm -rf /var/cache/apk/*

# Copy the compiled binary from builder stage
COPY --from=builder /bin/aigitcommit /bin/aigitcommit

# Create a non-root user for security
RUN addgroup -g 1000 aigit && \
    adduser -D -u 1000 -G aigit aigit

# Set the working directory
WORKDIR /repo

# Change ownership of the working directory
RUN chown -R aigit:aigit /repo

# Switch to non-root user
USER aigit

RUN aigitcommit --version

# Define the entrypoint
ENTRYPOINT ["/bin/aigitcommit"]

# Default command (can be overridden)
CMD ["--help"]
