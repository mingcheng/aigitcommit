# Stage 1: Dependencies - Cache Rust dependencies separately for faster rebuilds
FROM rust:alpine AS dependencies
LABEL maintainer="mingcheng <mingcheng@apache.org>"

# Install build dependencies required for compilation
RUN apk add --no-cache \
    build-base \
    git \
    musl-dev \
    libressl-dev \
    pkgconfig \
    perl

# Ensure we're using the latest stable Rust toolchain
RUN rustup default stable && rustup update stable

# Set the working directory for dependency building
WORKDIR /build

# Copy only dependency manifests first to leverage Docker layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy source file to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Stage 2: Builder - Build the actual application
FROM dependencies AS builder

# Copy the actual source code
COPY . .

# Build the application with optimizations
RUN cargo build --release && \
    strip target/release/aigitcommit && \
    cp target/release/aigitcommit /bin/aigitcommit

# Stage 3: Runtime - Create minimal runtime image
FROM alpine AS runtime

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

# Define the entrypoint
ENTRYPOINT ["/bin/aigitcommit"]

# Default command (can be overridden)
CMD ["--help"]
