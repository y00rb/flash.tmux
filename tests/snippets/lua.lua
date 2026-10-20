local function describe(value)
    if value > 0 then
        return {value = value, items = {1, 2}}
    end
    return {value = 0}
end
