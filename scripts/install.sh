#!/bin/bash

# Universal AI Development Assistant - Installation Script

set -e

echo "🚀 Installing Universal AI Development Assistant..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if running on supported OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    CYGWIN*)    MACHINE=Cygwin;;
    MINGW*)     MACHINE=MinGw;;
    *)          MACHINE="UNKNOWN:${OS}"
esac

if [ "$MACHINE" = "UNKNOWN:${OS}" ]; then
    echo -e "${RED}❌ Unsupported operating system: ${OS}${NC}"
    exit 1
fi

echo -e "${BLUE}🔍 Detected OS: ${MACHINE}${NC}"

# Check prerequisites
check_command() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}❌ $1 is not installed${NC}"
        return 1
    else
        echo -e "${GREEN}✅ $1 is installed${NC}"
        return 0
    fi
}

echo -e "${BLUE}🔍 Checking prerequisites...${NC}"

MISSING_DEPS=0

if ! check_command "curl"; then
    MISSING_DEPS=1
fi

if ! check_command "git"; then
    MISSING_DEPS=1
fi

if ! check_command "rustc"; then
    echo -e "${YELLOW}⚠️  Rust not found. Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    if check_command "rustc"; then
        echo -e "${GREEN}✅ Rust installed successfully${NC}"
    else
        echo -e "${RED}❌ Failed to install Rust${NC}"
        MISSING_DEPS=1
    fi
fi

if ! check_command "node"; then
    echo -e "${YELLOW}⚠️  Node.js not found. Please install Node.js 18+ from https://nodejs.org${NC}"
    MISSING_DEPS=1
fi

if [ $MISSING_DEPS -eq 1 ]; then
    echo -e "${RED}❌ Please install missing dependencies and try again${NC}"
    exit 1
fi

# Create installation directory
INSTALL_DIR="$HOME/.uaida"
echo -e "${BLUE}📁 Creating installation directory: ${INSTALL_DIR}${NC}"
mkdir -p "$INSTALL_DIR"

# Download and extract
echo -e "${BLUE}📥 Downloading UAIDA...${NC}"
cd "$INSTALL_DIR"

# Clone repository (in real deployment, this would download a release)
if [ -d "universal-ai-dev-assistant" ]; then
    echo -e "${YELLOW}⚠️  Existing installation found. Updating...${NC}"
    cd universal-ai-dev-assistant
    git pull
else
    git clone https://github.com/yourusername/universal-ai-dev-assistant.git
    cd universal-ai-dev-assistant
fi

# Build backend
echo -e "${BLUE}🔨 Building backend...${NC}"
cd backend
cargo build --release
cd ..

# Build frontend
echo -e "${BLUE}🔨 Building frontend...${NC}"
cd frontend
npm install
npm run build
cd ..

# Build CLI
echo -e "${BLUE}🔨 Building CLI...${NC}"
cd cli
cargo build --release
cd ..

# Create symlinks
echo -e "${BLUE}🔗 Creating symlinks...${NC}"
sudo ln -sf "$INSTALL_DIR/universal-ai-dev-assistant/cli/target/release/uaida" /usr/local/bin/uaida
sudo ln -sf "$INSTALL_DIR/universal-ai-dev-assistant/backend/target/release/universal-ai-dev-assistant" /usr/local/bin/uaida-server

# Create systemd service (Linux only)
if [ "$MACHINE" = "Linux" ]; then
    echo -e "${BLUE}⚙️  Creating systemd service...${NC}"
    sudo tee /etc/systemd/system/uaida.service > /dev/null <<EOF
[Unit]
Description=Universal AI Development Assistant
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$INSTALL_DIR/universal-ai-dev-assistant
ExecStart=/usr/local/bin/uaida-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable uaida
fi

# Install VS Code extension
if command -v code &> /dev/null; then
    echo -e "${BLUE}📦 Installing VS Code extension...${NC}"
    cd extensions/vscode
    npm install
    npm run package
    code --install-extension *.vsix
    cd ../..
else
    echo -e "${YELLOW}⚠️  VS Code not found. Skipping extension installation.${NC}"
fi

# Create configuration
echo -e "${BLUE}⚙️  Creating configuration...${NC}"
mkdir -p "$HOME/.config/uaida"
cat > "$HOME/.config/uaida/config.toml" <<EOF
[server]
host = "127.0.0.1"
port = 8080

[ai]
model_name = "codellama-7b-instruct"
max_tokens = 2048
temperature = 0.1

[database]
url = "sqlite://$HOME/.config/uaida/uaida.db"
EOF

echo -e "${GREEN}🎉 Installation completed successfully!${NC}"
echo ""
echo -e "${BLUE}📖 Quick Start:${NC}"
echo -e "  • Start server: ${YELLOW}uaida server${NC}"
echo -e "  • Analyze code: ${YELLOW}uaida analyze your-file.py${NC}"
echo -e "  • Web interface: ${YELLOW}http://localhost:8080${NC}"
echo -e "  • Documentation: ${YELLOW}uaida --help${NC}"
echo ""
echo -e "${BLUE}🔧 Configuration file: ${YELLOW}$HOME/.config/uaida/config.toml${NC}"
echo ""
echo -e "${GREEN}Happy coding with UAIDA! 🚀${NC}"