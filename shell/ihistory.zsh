# ihistory - Zsh integration
# Usage: eval "$(ihistory --init zsh)"

# Remove old alias if it exists
unalias ih 2>/dev/null

# ih function - interactive history search
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

ih-widget() {
  local selected
  local saved_buffer="$BUFFER"
  local saved_cursor="$CURSOR"

  selected="$(ihistory)"
  local ret=$?

  zle reset-prompt

  if [[ $ret -eq 0 && -n "$selected" ]]; then
    BUFFER="$selected"
    CURSOR=${#BUFFER}
  elif [[ $ret -eq 10 && -n "$selected" ]]; then
    BUFFER="$selected"
    zle accept-line
    return
  else
    BUFFER="$saved_buffer"
    CURSOR="$saved_cursor"
  fi

  zle redisplay
}

zle -N ih-widget
