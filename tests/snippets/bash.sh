describe() {
    local value="$1"
    if [ "$value" -gt 0 ]; then
        printf "%s\n" "$value"
    fi
}
