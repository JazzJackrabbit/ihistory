# ihistory - Shell integration
# Add to ~/.zshrc or ~/.bashrc:
#   source /path/to/ihistory.sh

if [ -n "$ZSH_VERSION" ]; then
  # Zsh
  unalias ih 2>/dev/null

  ih() {
    local selected ret
    selected="$(ihistory "$@")"
    ret=$?
    if [[ -n "$selected" ]]; then
      if [[ $ret -eq 10 ]]; then
        eval "$selected"
      else
        print -z "$selected"
      fi
    fi
  }

elif [ -n "$BASH_VERSION" ]; then
  # Bash
  ih() {
    local selected ret
    selected="$(ihistory "$@")"
    ret=$?
    if [[ -n "$selected" ]]; then
      if [[ $ret -eq 10 ]]; then
        eval "$selected"
      else
        history -s "$selected"
        echo "$selected"
      fi
    fi
  }
fi
