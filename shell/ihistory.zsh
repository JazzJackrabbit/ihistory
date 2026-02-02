# ihistory - Zsh integration
# Add to your ~/.zshrc:
#   source /path/to/ihistory.zsh
#
# This creates an `ih` function and binds Ctrl+R to ihistory

# Remove old alias if it exists
unalias ih 2>/dev/null

# ih function - interactive history search
# Enter = copy to buffer, Tab = execute immediately
ih() {
  local selected ret
  selected="$(ihistory "$@")"
  ret=$?
  if [[ -n "$selected" ]]; then
    if [[ $ret -eq 10 ]]; then
      # Tab pressed - execute immediately
      eval "$selected"
    else
      # Enter pressed - copy to buffer for editing
      print -z "$selected"
    fi
  fi
}

ih-widget() {
  local selected
  # Save current buffer
  local saved_buffer="$BUFFER"
  local saved_cursor="$CURSOR"

  # Run ihistory
  selected="$(ihistory)"
  local ret=$?

  # Restore terminal state
  zle reset-prompt

  if [[ $ret -eq 0 && -n "$selected" ]]; then
    # Exit 0 = put in buffer for editing
    BUFFER="$selected"
    CURSOR=${#BUFFER}
  elif [[ $ret -eq 10 && -n "$selected" ]]; then
    # Exit 10 = execute immediately
    BUFFER="$selected"
    zle accept-line
    return
  else
    # Restore original buffer on cancel
    BUFFER="$saved_buffer"
    CURSOR="$saved_cursor"
  fi

  zle redisplay
}

# Register the widget
zle -N ih-widget

# Bind to Ctrl+R (replace default reverse history search)
bindkey '^R' ih-widget
