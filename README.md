# Differentially private integer, nonnegative histograms demonstration
*author:* Christina Ilvento based on joint work with Cynthia Dwork

## Overview
There are many settings in which it is important to produce differentially private approximations to histograms with integer nonnegative counts. 
In this repository, we include the supplemental material for upcoming work describing a novel method for generating differentially private, integer, nonnegative histograms without integer programming. 
The basic intuition behind this approach is to view histogram is a list of counts in order from largest to smallest and a list of the corresponding sorted cell names. 
This decomposition allows us to solve two more manageable problems: differentially private integer partitions, and differentially private "re-attribution" or "sorting". 


To demonstrate the technique, we implement several variants of the method (`ip_demo/src/main.rs`) and apply them to the Social Security Administration [baby names data set](https://www.ssa.gov/OACT/babynames/limits.html) (1880-1885) in the accompanying .ipython notebook (`demo.ipynb`).
We implement the two-stage procedure exactly using the code for private integer partitions, exponential mechanism, etc from [github.com/cilvento/b2dp](https://github.com/cilvento/b2dp).

**Build instructions**:
To reproduce our results (or try it out for yourself on other data) see [`demo.ipynb`](./demo.ipynb) for example invocations. To build the `ip_demo` binary, you will need a local copy of the [`b2dp` crate](https://github.com/cilvento/b2dp) and its dependencies. You may need to modify the paths to dependencies in `cargo.toml`. 
