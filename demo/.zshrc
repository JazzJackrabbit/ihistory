PS1="$ "

ih() {
  local selected ret
  local script_dir="${0:A:h}"
  selected="$(cargo run --quiet --manifest-path "${script_dir}/../Cargo.toml" -- "$@")"
  ret=$?
  if [[ -n "$selected" ]]; then
    if [[ $ret -eq 10 ]]; then
      echo "$ $selected"
      eval "$selected"
    else
      echo "$ $selected"
    fi
  fi
}
