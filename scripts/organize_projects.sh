#!/bin/zsh

# Get the user's home directory in a reliable way
USER_HOME=$(eval echo ~)

# Create the Developer directory if it doesn't exist
mkdir -p "$USER_HOME/Developer"

# List of folders to exclude from search
EXCLUDE_DIRS=("Developer" "Library" "Applications" ".Trash" ".zshrc" ".bashrc" ".config" ".local" ".ssh" ".npm" ".cargo" ".rustup" ".pyenv" ".rbenv" ".gem" ".bundle" ".vscode" ".git" ".DS_Store")

# Loop through each item in the home directory
for dir in "$USER_HOME"/*; do
  # Check if it's a directory
  if [ -d "$dir" ]; then
    # Get the base name
    base=$(basename "$dir")
    # Skip excluded directories
    if [[ " ${EXCLUDE_DIRS[@]} " =~ " $base " ]]; then
      continue
    fi
    # Check for project conditions
    if [ -d "$dir/.git" ] || [ -f "$dir/.gitignore" ] || \
       [ -n "$(find "$dir" -maxdepth 2 -type f \( -iname '*.js' -o -iname '*.ts' -o -iname '*.py' -o -iname '*.rs' -o -iname '*.go' -o -iname '*.java' -o -iname '*.swift' \) -print -quit)" ]; then
      echo "Moving project: $(basename "$dir")"
      # Move the directory without overwriting
      mv -n "$dir" "$USER_HOME/Developer/"
    fi
  fi
done

echo "Home folder organization complete."
