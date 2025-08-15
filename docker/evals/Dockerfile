# Dockerfile for evaluation runner
FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js for JavaScript evaluations
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - \
    && apt-get install -y nodejs

# Install Rust for Rust evaluations
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /app

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install -r requirements.txt

# Copy evaluation scripts
COPY scripts/evals/ ./scripts/evals/
COPY data/evals/ ./data/evals/

# Make scripts executable
RUN chmod +x scripts/evals/*.sh

# Default command
CMD ["python", "scripts/evals/run_humaneval.py", "--help"]