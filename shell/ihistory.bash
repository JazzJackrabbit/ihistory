# ihistory - Bash integration
# Add to your ~/.bashrc:
#   source /path/to/ihistory.bash
#
# This creates an `ih` function and binds Ctrl+R to ihistory

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
      history -s "$selected"
      echo "$selected"
    fi
  fi
}

ih-widget() {
  local selected

  # Run ihistory
  selected="$(ihistory)"
  local ret=$?

  if [[ $ret -eq 0 && -n "$selected" ]]; then
    # Insert selected command into readline buffer
    READLINE_LINE="$selected"
    READLINE_POINT=${#selected}
  elif [[ $ret -eq 10 && -n "$selected" ]]; then
    # Exit 10 = execute immediately
    READLINE_LINE="$selected"
    READLINE_POINT=${#selected}
  fi
}

# Bind to Ctrl+R (replace default reverse history search)
bind -x '"\C-r": ih-widget'
