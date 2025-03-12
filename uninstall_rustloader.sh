#!/bin/bash

# Rustloader Uninstallation Script
# This script removes Rustloader but preserves yt-dlp and ffmpeg

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Rustloader Uninstallation Script    ${NC}"
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

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to handle script interruption
handle_interrupt() {
    echo -e "\n${YELLOW}Uninstallation interrupted. Some components may not have been completely removed.${NC}"
    exit 1
}

# Set up trap for interruptions
trap handle_interrupt INT TERM

# Get confirmation for actions
get_confirmation() {
    local message=$1
    local default=${2:-n}
    
    if [[ "$default" == "y" ]]; then
        read -p "$message (Y/n): " CONFIRM
        [[ -z "$CONFIRM" || "$CONFIRM" =~ ^[Yy] ]]
    else
        read -p "$message (y/N): " CONFIRM
        [[ "$CONFIRM" =~ ^[Yy]$ ]]
    fi
}

# Function to remove the rustloader binary
remove_rustloader_binary() {
    echo -e "${YELLOW}Searching for Rustloader binary...${NC}"
    
    # Try to find rustloader in common locations
    local binary_locations=(
        "$(which rustloader 2>/dev/null || echo '')"
        "$HOME/.cargo/bin/rustloader"
        "/usr/local/bin/rustloader"
        "/usr/bin/rustloader"
        "$HOME/.local/bin/rustloader"
    )
    
    local found=false
    
    for location in "${binary_locations[@]}"; do
        if [[ -f "$location" ]]; then
            echo -e "${GREEN}Found Rustloader binary at: $location${NC}"
            
            if get_confirmation "Remove this binary?"; then
                if [[ "$location" == "/usr/local/bin/rustloader" || "$location" == "/usr/bin/rustloader" ]]; then
                    echo -e "${YELLOW}This location may require sudo privileges.${NC}"
                    sudo rm -f "$location" || { 
                        echo -e "${RED}Failed to remove binary. Please try: sudo rm -f $location${NC}"; 
                        continue;
                    }
                else
                    rm -f "$location" || { 
                        echo -e "${RED}Failed to remove binary at $location${NC}"; 
                        continue;
                    }
                fi
                
                echo -e "${GREEN}Successfully removed Rustloader binary from $location${NC}"
                found=true
            else
                echo -e "${YELLOW}Skipping removal of Rustloader binary at $location${NC}"
            fi
        fi
    done
    
    # Check for cargo installation
    if command_exists cargo; then
        if cargo install --list | grep -q "rustloader"; then
            echo -e "${GREEN}Found Rustloader in Cargo registry${NC}"
            
            if get_confirmation "Uninstall Rustloader via Cargo?"; then
                cargo uninstall rustloader
                echo -e "${GREEN}Successfully uninstalled Rustloader via Cargo${NC}"
                found=true
            else
                echo -e "${YELLOW}Skipping Cargo uninstallation${NC}"
            fi
        fi
    fi
    
    if [[ "$found" == "false" ]]; then
        echo -e "${YELLOW}Rustloader binary not found in common locations.${NC}"
        echo -e "${YELLOW}If you installed it in a custom location, you'll need to remove it manually.${NC}"
    fi
}

# Function to remove configuration files
remove_config_files() {
    echo -e "${YELLOW}Checking for configuration files...${NC}"
    
    local config_dirs=(
        "$HOME/.config/rustloader"
        "$HOME/Library/Application Support/rustloader"
        "$HOME/Library/Preferences/rustloader"
        "$APPDATA/rustloader"
        "$HOME/.rustloader"
    )
    
    local found=false
    
    for dir in "${config_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            echo -e "${GREEN}Found configuration directory: $dir${NC}"
            
            if get_confirmation "Remove this configuration directory?"; then
                rm -rf "$dir" || echo -e "${RED}Failed to remove $dir${NC}"
                echo -e "${GREEN}Removed configuration directory: $dir${NC}"
                found=true
            else
                echo -e "${YELLOW}Keeping configuration directory: $dir${NC}"
            fi
        fi
    done
    
    if [[ "$found" == "false" ]]; then
        echo -e "${YELLOW}No configuration directories found.${NC}"
    fi
}

# Function to remove data files
remove_data_files() {
    echo -e "${YELLOW}Checking for data files...${NC}"
    
    local data_dirs=(
        "$HOME/.local/share/rustloader"
        "$HOME/Library/Application Support/rustloader"
        "$APPDATA/rustloader"
        "$HOME/.cache/rustloader"
        "$LOCALAPPDATA/rustloader"
    )
    
    local found=false
    
    for dir in "${data_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            echo -e "${GREEN}Found data directory: $dir${NC}"
            
            if get_confirmation "Remove this data directory?"; then
                rm -rf "$dir" || echo -e "${RED}Failed to remove $dir${NC}"
                echo -e "${GREEN}Removed data directory: $dir${NC}"
                found=true
            else
                echo -e "${YELLOW}Keeping data directory: $dir${NC}"
            fi
        fi
    done
    
    if [[ "$found" == "false" ]]; then
        echo -e "${YELLOW}No data directories found.${NC}"
    fi
}

# Function to check if user wants to remove downloaded content
remove_downloads() {
    echo -e "${YELLOW}Checking for downloaded content...${NC}"
    
    local download_dirs=(
        "$HOME/Downloads/rustloader"
    )
    
    # Check for custom download path in config
    local custom_download_paths=(
        "$HOME/.config/rustloader/download_path"
        "$HOME/Library/Preferences/rustloader/download_path"
        "$APPDATA/rustloader/download_path"
    )
    
    for path_file in "${custom_download_paths[@]}"; do
        if [[ -f "$path_file" ]]; then
            custom_path=$(cat "$path_file")
            if [[ -n "$custom_path" && -d "$custom_path" ]]; then
                download_dirs+=("$custom_path")
            fi
        fi
    done
    
    local found=false
    
    for dir in "${download_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            echo -e "${GREEN}Found download directory: $dir${NC}"
            
            # Show size if possible
            if command_exists du; then
                dir_size=$(du -sh "$dir" 2>/dev/null | cut -f1)
                echo -e "${YELLOW}Directory size: $dir_size${NC}"
            fi
            
            if get_confirmation "Remove downloaded content? This will delete all downloaded files."; then
                rm -rf "$dir" || echo -e "${RED}Failed to remove $dir${NC}"
                echo -e "${GREEN}Removed download directory: $dir${NC}"
                found=true
            else
                echo -e "${YELLOW}Keeping download directory: $dir${NC}"
            fi
        fi
    done
    
    if [[ "$found" == "false" ]]; then
        echo -e "${YELLOW}No download directories found.${NC}"
    fi
}

# Function to remove license files
remove_license_files() {
    echo -e "${YELLOW}Checking for license files...${NC}"
    
    local license_files=(
        "$HOME/.config/rustloader/license.dat"
        "$HOME/.local/share/rustloader/license.dat"
        "$HOME/Library/Application Support/rustloader/license.dat"
        "$HOME/Library/Preferences/rustloader/license.dat"
        "$APPDATA/rustloader/license.dat"
    )
    
    local found=false
    
    for file in "${license_files[@]}"; do
        if [[ -f "$file" ]]; then
            echo -e "${GREEN}Found license file: $file${NC}"
            
            if get_confirmation "Remove license file?"; then
                rm -f "$file" || echo -e "${RED}Failed to remove $file${NC}"
                echo -e "${GREEN}Removed license file: $file${NC}"
                found=true
            else
                echo -e "${YELLOW}Keeping license file: $file${NC}"
            fi
        fi
    done
    
    if [[ "$found" == "false" ]]; then
        echo -e "${YELLOW}No license files found.${NC}"
    fi
}

# Function to check for any leftover files
check_leftover_files() {
    echo -e "${YELLOW}Checking for any leftover files...${NC}"
    
    local leftover_dirs=(
        "$HOME/.rustloader"
        "$HOME/.config/rustloader"
        "$HOME/.local/share/rustloader"
        "$HOME/Library/Application Support/rustloader"
        "$HOME/Library/Preferences/rustloader"
        "$APPDATA/rustloader"
        "$LOCALAPPDATA/rustloader"
        "$HOME/.cache/rustloader"
    )
    
    local found=false
    local leftover_locations=()
    
    for dir in "${leftover_dirs[@]}"; do
        if [[ -e "$dir" ]]; then
            leftover_locations+=("$dir")
            found=true
        fi
    done
    
    if [[ "$found" == "true" ]]; then
        echo -e "${YELLOW}Found leftover files in these locations:${NC}"
        for location in "${leftover_locations[@]}"; do
            echo "  - $location"
        done
        
        if get_confirmation "Remove all leftover files?"; then
            for location in "${leftover_locations[@]}"; do
                rm -rf "$location" || echo -e "${RED}Failed to remove $location${NC}"
                echo -e "${GREEN}Removed leftover files: $location${NC}"
            done
        else
            echo -e "${YELLOW}Keeping leftover files${NC}"
        fi
    else
        echo -e "${GREEN}No leftover files found.${NC}"
    fi
}

# Function to display completion message
display_completion_message() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}    Rustloader has been uninstalled    ${NC}"
    echo -e "${GREEN}========================================${NC}"
    
    echo -e "${YELLOW}Note: yt-dlp and ffmpeg were kept as requested${NC}"
    echo -e "${YELLOW}These dependencies are still installed and can be used by other applications${NC}"
    
    echo -e "${BLUE}If you want to remove yt-dlp and ffmpeg as well:${NC}"
    
    case $OS in
        "Linux")
            echo -e "  - To remove yt-dlp: ${GREEN}pip uninstall yt-dlp${NC}"
            echo -e "  - To remove ffmpeg: ${GREEN}sudo apt remove ffmpeg${NC} (or equivalent for your distribution)"
            ;;
        "macOS")
            echo -e "  - To remove yt-dlp and ffmpeg: ${GREEN}brew uninstall yt-dlp ffmpeg${NC}"
            ;;
        "Windows")
            echo -e "  - To remove yt-dlp: ${GREEN}pip uninstall yt-dlp${NC}"
            echo -e "  - To remove ffmpeg: Use Windows Add/Remove Programs or ${GREEN}choco uninstall ffmpeg${NC}"
            ;;
        *)
            echo -e "  - To remove yt-dlp: ${GREEN}pip uninstall yt-dlp${NC}"
            echo -e "  - To remove ffmpeg: Use your system's package manager"
            ;;
    esac
    
    echo -e "\n${BLUE}Thank you for trying Rustloader!${NC}"
}

# Main uninstallation function
main() {
    echo -e "${BLUE}Starting uninstallation process...${NC}"
    
    # Confirm uninstallation
    if ! get_confirmation "Are you sure you want to uninstall Rustloader? yt-dlp and ffmpeg will be preserved."; then
        echo -e "${YELLOW}Uninstallation cancelled.${NC}"
        exit 0
    fi
    
    # Perform each uninstallation step
    remove_rustloader_binary
    remove_config_files
    remove_data_files
    remove_downloads
    remove_license_files
    check_leftover_files
    display_completion_message
}

# Run the main function
main