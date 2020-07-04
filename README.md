# rand-facade

An experimental global facade for `rand::RngCore` to support use of initialised random number generators in `no_std` capable libraries and projects, without needing to specify a given random generator within the library.

This allows you to initialise and maintain a physical Random Number Generator (RNG) on `no_std` platforms, while allowing the sharing a global RNG (if required) or falling through to the default `OsRng` on `std` platforms.

This is intended to be used as a dependency for relevant libraries / projects that require RNGs, and allows modes to be swapped using the feature flags described below.


## Usage

Include by adding `rand-facade = "0.1.0"` to your `Cargo.toml`.

### Features

- `os_rng` disables binding and falls through to the default `rand::rng::OsRng`, this is a sensible default for most apps
- `std` enables global `Rng` binding using `std::sync::Mutex`
- `cortex_m` enables global `Rng` binding using `cortex_m::Mutex`

## Status

This is a work in progress! Currently this works with `std` and `cortex-m` platforms.

[![GitHub tag](https://img.shields.io/github/tag/ryankurte/rust-rand-facade.svg)](https://github.com/ryankurte/rust-rand-facade)
[![Build Status](https://travis-ci.com/ryankurte/rust-rand-facade.svg?branch=master)](https://travis-ci.com/ryankurte/rust-rand-facade)
[![Crates.io](https://img.shields.io/crates/v/rand-facade.svg)](https://crates.io/crates/rand-facade)
[![Docs.rs](https://docs.rs/rand-facade/badge.svg)](https://docs.rs/rand-facade)

[Open Issues](https://github.com/ryankurte/rust-rand-facade/issues)
