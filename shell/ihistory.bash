# ihistory - Bash integration
# Usage: eval "$(ihistory --init bash)"

# ih function - interactive history search
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
