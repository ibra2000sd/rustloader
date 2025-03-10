#!/bin/bash

# Rustloader Installation Script
# This script installs rustloader and all its dependencies

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Rustloader Installation Script      ${NC}"
echo -e "${BLUE}========================================${NC}"

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="Linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macOS"
elif [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="Windows"
else
    OS="Unknown"
fi

echo -e "${GREEN}Detected operating system: ${OS}${NC}"

# Check if running with sudo/root on Linux
if [[ "$OS" == "Linux" ]] && [[ $EUID -ne 0 ]]; then
    echo -e "${YELLOW}Warning: This script may need sudo privileges to install dependencies.${NC}"
    echo -e "${YELLOW}If it fails, please run it again with sudo.${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Installation cancelled.${NC}"
        exit 1
    fi
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install Rust if not installed
install_rust() {
    if ! command_exists cargo; then
        echo -e "${YELLOW}Rust is not installed. Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo -e "${GREEN}Rust installed successfully.${NC}"
    else
        echo -e "${GREEN}Rust is already installed.${NC}"
    fi
}

# Install dependencies based on OS
install_dependencies() {
    echo -e "${BLUE}Installing dependencies...${NC}"
    
    case $OS in
        "Linux")
            # Detect package manager
            if command_exists apt; then
                echo -e "${YELLOW}Using apt package manager...${NC}"
                sudo apt update
                sudo apt install -y python3 python3-pip ffmpeg libssl-dev pkg-config
            elif command_exists dnf; then
                echo -e "${YELLOW}Using dnf package manager...${NC}"
                sudo dnf install -y python3 python3-pip ffmpeg openssl-devel pkgconfig
            elif command_exists pacman; then
                echo -e "${YELLOW}Using pacman package manager...${NC}"
                sudo pacman -Sy python python-pip ffmpeg openssl pkg-config
            else
                echo -e "${RED}Unsupported Linux distribution. Please install Python, pip, ffmpeg, openssl-dev and pkg-config manually.${NC}"
                exit 1
            fi
            
            # Install yt-dlp
            pip3 install --user --upgrade yt-dlp
            ;;
            
        "macOS")
            if ! command_exists brew; then
                echo -e "${YELLOW}Homebrew not found. Installing Homebrew...${NC}"
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            fi
            
            echo -e "${YELLOW}Installing dependencies with Homebrew...${NC}"
            brew install python ffmpeg yt-dlp openssl@3 pkg-config
            
            # Set up environment for OpenSSL
            export OPENSSL_DIR=$(brew --prefix openssl@3)
            echo "export OPENSSL_DIR=$(brew --prefix openssl@3)" >> ~/.bash_profile
            echo "export OPENSSL_DIR=$(brew --prefix openssl@3)" >> ~/.zshrc
            ;;
            
        "Windows")
            echo -e "${YELLOW}Windows detected. This script has limited support for Windows.${NC}"
            echo -e "${YELLOW}Installing Python and pip...${NC}"
            if ! command_exists choco; then
                echo -e "${RED}Chocolatey not found. Please install it first: https://chocolatey.org/install${NC}"
                echo -e "${YELLOW}Or install Python, ffmpeg, and yt-dlp manually.${NC}"
                exit 1
            fi
            
            choco install -y python ffmpeg openssl
            pip install --user --upgrade yt-dlp
            ;;
            
        *)
            echo -e "${RED}Unsupported operating system: $OS${NC}"
            echo -e "${YELLOW}Please install Python, ffmpeg, and yt-dlp manually.${NC}"
            exit 1
            ;;
    esac
    
    echo -e "${GREEN}Dependencies installed successfully.${NC}"
}

# Verify dependencies
verify_dependencies() {
    echo -e "${BLUE}Verifying dependencies...${NC}"
    
    local missing=0
    
    if ! command_exists yt-dlp; then
        echo -e "${RED}yt-dlp not found in PATH${NC}"
        missing=1
    else
        echo -e "${GREEN}yt-dlp is installed: $(yt-dlp --version 2>&1 | head -n 1)${NC}"
    fi
    
    if ! command_exists ffmpeg; then
        echo -e "${RED}ffmpeg not found in PATH${NC}"
        missing=1
    else
        echo -e "${GREEN}ffmpeg is installed: $(ffmpeg -version 2>&1 | head -n 1)${NC}"
    fi
    
    if [[ $missing -eq 1 ]]; then
        echo -e "${YELLOW}Some dependencies are missing. Please install them manually and run this script again.${NC}"
        return 1
    fi
    
    return 0
}

# Install rustloader
install_rustloader() {
    echo -e "${BLUE}Installing rustloader...${NC}"
    
    # Create a temporary directory
    TMP_DIR=$(mktemp -d)
    echo -e "${YELLOW}Created temporary directory: $TMP_DIR${NC}"
    
    # Clone the repository
    echo -e "${YELLOW}Cloning rustloader repository...${NC}"
    git clone https://github.com/ibra2000sd/rustloader.git "$TMP_DIR/rustloader"
    
    # Build and install
    echo -e "${YELLOW}Building and installing rustloader...${NC}"
    cd "$TMP_DIR/rustloader"
    cargo install --path .
    
    # Clean up
    echo -e "${YELLOW}Cleaning up...${NC}"
    cd - > /dev/null
    rm -rf "$TMP_DIR"
    
    echo -e "${GREEN}Rustloader installed successfully.${NC}"
}

# Add rustloader to PATH if not already there
setup_path() {
    echo -e "${BLUE}Setting up PATH...${NC}"
    
    # Check if ~/.cargo/bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.cargo/bin:"* ]]; then
        echo -e "${YELLOW}Adding ~/.cargo/bin to PATH...${NC}"
        
        # Detect shell
        SHELL_NAME=$(basename "$SHELL")
        
        case $SHELL_NAME in
            "bash")
                PROFILE_FILE="$HOME/.bashrc"
                ;;
            "zsh")
                PROFILE_FILE="$HOME/.zshrc"
                ;;
            *)
                PROFILE_FILE="$HOME/.profile"
                ;;
        esac
        
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$PROFILE_FILE"
        echo -e "${GREEN}Added ~/.cargo/bin to PATH in $PROFILE_FILE${NC}"
        echo -e "${YELLOW}Please run 'source $PROFILE_FILE' or restart your terminal to apply changes.${NC}"
    else
        echo -e "${GREEN}~/.cargo/bin is already in PATH.${NC}"
    fi
}

# Test installation
test_installation() {
    echo -e "${BLUE}Testing rustloader installation...${NC}"
    
    if command_exists rustloader; then
        echo -e "${GREEN}Rustloader is installed and in PATH.${NC}"
        echo -e "${YELLOW}Rustloader version:${NC}"
        rustloader --version
        echo -e "${GREEN}Installation successful!${NC}"
    else
        echo -e "${RED}Rustloader is not in PATH. Please restart your terminal or add ~/.cargo/bin to your PATH manually.${NC}"
        echo -e "${RED}Installation may have failed.${NC}"
        exit 1
    fi
}

# Clean up existing download counter if it exists
cleanup_existing_data() {
    echo -e "${BLUE}Checking for existing data...${NC}"
    
    # Define paths based on OS
    local data_dir=""
    if [[ "$OS" == "Linux" ]]; then
        data_dir="$HOME/.local/share/rustloader"
    elif [[ "$OS" == "macOS" ]]; then
        data_dir="$HOME/Library/Application Support/rustloader"
    elif [[ "$OS" == "Windows" ]]; then
        data_dir="$APPDATA/rustloader"
    fi
    
    if [[ -d "$data_dir" ]]; then
        echo -e "${YELLOW}Found existing rustloader data directory: $data_dir${NC}"
        read -p "Would you like to clean up old data files? (Recommended for updates) (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo -e "${YELLOW}Removing old data files...${NC}"
            rm -f "$data_dir/download_counter.dat"
            echo -e "${GREEN}Old data files removed.${NC}"
        fi
    fi
}

# Display final instructions
display_instructions() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${GREEN}Rustloader has been successfully installed!${NC}"
    echo -e "${GREEN}You can now use it by running 'rustloader' in your terminal.${NC}"
    echo -e "${YELLOW}Basic Usage:${NC}"
    echo -e "  rustloader [URL] [OPTIONS]"
    echo -e "${YELLOW}Examples:${NC}"
    echo -e "  rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ                  # Download video"
    echo -e "  rustloader https://www.youtube.com/watch?v=dQw4w9WgXcQ --format mp3     # Download audio"
    echo -e "  rustloader --help                                                        # Show all options"
    echo -e "${YELLOW}Pro Version:${NC}"
    echo -e "  rustloader --activate YOUR_LICENSE_KEY                                   # Activate Pro"
    echo -e "  rustloader --license                                                     # Show license info"
    echo -e "${BLUE}========================================${NC}"
}

# Main installation process
main() {
    echo -e "${BLUE}Starting installation...${NC}"
    
    install_rust
    install_dependencies
    
    if ! verify_dependencies; then
        echo -e "${RED}Dependency verification failed. Exiting.${NC}"
        exit 1
    fi
    
    install_rustloader
    setup_path
    cleanup_existing_data
    test_installation
    display_instructions
}

# Run the installation
main
