function describe(value: number): object {
    if (value > 0) {
        return {value: value, items: [1, 2]};
    }
    return {value: 0};
}
