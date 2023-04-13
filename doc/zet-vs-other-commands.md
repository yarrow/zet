 Here are results from [`hyperfine`](https://github.com/sharkdp/hyperfine) benchmarks of `zet` and some other commands on a 2020 Macbook Air M1. The rows annotated "with max allocations" ran `zet` with `/dev/null` prepended to its argument list — `zet` is a little faster if most of the lines of its output orginate in its first argument, because it borrows each line of that first argument as an [`IndexMap`](https://github.com/bluss/indexmap/blob/master/README.md) key, but does an allocation for each line of subsequent arguments (when the line doesn't already exist in the `IndexMap`).

 Command                    | Mean [s]       | Min [s]| Max [s]| Relative 
:---                        |:---            |:---    |:---    |:---      
 `zet union`                | 1.197 ± 0.029  | 1.141  | 1.234  | 1.00 
 `... with max allocations` | 1.256 ± 0.015  | 1.230  | 1.278  | 1.05 ± 0.03 
 `uniq`                     | 5.931 ± 0.065  | 5.839  | 6.065  | 4.95 ± 0.13 

 Command                    | Mean [s]       | Min [s]| Max [s]| Relative 
:---                        |:---            |:---    |:---    |:---      
 `zet union -c`             | 1.253 ± 0.020  | 1.229  | 1.289  | 1.00 
 `... with max allocations` | 1.320 ± 0.020  | 1.290  | 1.354  | 1.05 ± 0.02 
 `uniq -c`                  | 6.091 ± 0.052  | 6.006  | 6.167  | 4.86 ± 0.09 

 Command                    | Mean [s]       | Min [s]| Max [s]| Relative 
:---                        |:---            |:---    |:---    |:---      
 `zet single`               | 1.230 ± 0.029  | 1.201  | 1.294  | 1.00 
 `... with max allocations` | 1.249 ± 0.019  | 1.229  | 1.290  | 1.02 ± 0.03 
 `uniq -u`                  | 5.947 ± 0.049  | 5.897  | 6.071  | 4.84 ± 0.12 

 Command                    | Mean [s]       | Min [s]| Max [s]| Relative 
:---                        |:---            |:---    |:---    |:---      
 `zet multiple`             | 1.151 ± 0.019  | 1.127  | 1.184  | 1.00 
 `... with max allocations` | 1.348 ± 0.022  | 1.306  | 1.382  | 1.17 ± 0.03 
 `uniq -d`                  | 5.752 ± 0.045  | 5.696  | 5.851  | 5.00 ± 0.09 

The `/dev/null` trick doesn't work for `zet intersect` and `zet diff`, since it would result in empty output for both commands.

 Command                    | Mean [s]       | Min [s]| Max [s]| Relative 
:---                        |:---            |:---    |:---    |:---      
 `zet intersect`            | 1.950 ± 0.029  | 1.920  | 2.020  | 1.00 
 `comm -12`                 | 21.631 ± 0.083 | 21.504 | 21.762 | 11.09 ± 0.17 
 `zet diff`                 | 1.889 ± 0.030  | 1.843  | 1.930  | 1.00 
 `comm -23`                 | 20.038 ± 0.100 | 19.892 | 20.221 | 10.61 ± 0.18 

 `huniq` is somewhat faster than `zet` when line counts aren't involved, and somewhat slower when they are.

 Command                    | Mean [s]       | Min [s]| Max [s]| Relative 
:---                        |:---            |:---    |:---    |:---      
 `zet union`                | 2.963 ± 0.037  | 2.890  | 3.006  | 1.24 ± 0.02 
 `huniq`                    | 2.383 ± 0.015  | 2.367  | 2.417  | 1.00 
 `zet union -c`             | 3.016 ± 0.031  | 2.971  | 3.071  | 1.00 
 `huniq -c`                 | 3.628 ± 0.043  | 3.568  | 3.700  | 1.20 ± 0.02 
