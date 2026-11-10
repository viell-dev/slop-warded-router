BEGIN { FS = OFS = "," }
FNR > 1 {
    key = $2 SUBSEP $3
    count[key]++
    value[key, count[key]] = $4 + 0
}
END {
    print "scenario,approach,median_ns,min_process_median_ns,max_process_median_ns"
    for (key in count) {
        if (count[key] != 3) {
            print "Expected exactly three process medians" > "/dev/stderr"
            exit 1
        }
        a = value[key, 1]; b = value[key, 2]; c = value[key, 3]
        low = a < b ? a : b; low = low < c ? low : c
        high = a > b ? a : b; high = high > c ? high : c
        split(key, parts, SUBSEP)
        printf "%s,%s,%.3f,%.3f,%.3f\n", parts[1], parts[2], a+b+c-low-high, low, high
    }
}
