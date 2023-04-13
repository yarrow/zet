Here are results from [`hyperfine`](https://github.com/sharkdp/hyperfine) benchmarks of `zet` and `zet 0.2.0` version on a 2020 Macbook Air M1. The `0.2.0` version was the only version before `1.0.0` with a public announcement.

| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `zet union` | 2.927 ± 0.041 | 2.883 | 2.989 | 1.00 |
| `./zet-0.2.0 union` | 5.019 ± 0.045 | 4.947 | 5.077 | 1.71 ± 0.03 |
| `zet intersect` | 2.670 ± 0.040 | 2.602 | 2.728 | 1.00 |
| `./zet-0.2.0 intersect` | 4.762 ± 0.042 | 4.696 | 4.828 | 1.78 ± 0.03 |
| `zet diff` | 2.647 ± 0.032 | 2.577 | 2.679 | 1.00 |
| `./zet-0.2.0 diff` | 4.127 ± 0.058 | 4.043 | 4.218 | 1.56 ± 0.03 |
| `zet single --file` | 2.931 ± 0.020 | 2.893 | 2.959 | 1.00 |
| `./zet-0.2.0 single` | 7.096 ± 0.181 | 6.949 | 7.583 | 2.42 ± 0.06 |
| `zet multiple --file` | 2.971 ± 0.051 | 2.890 | 3.049 | 1.00 |
| `./zet-0.2.0 multiple` | 7.220 ± 0.076 | 7.126 | 7.306 | 2.43 ± 0.05 |
