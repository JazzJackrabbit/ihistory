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
