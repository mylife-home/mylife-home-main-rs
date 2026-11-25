# Every target covers the whole workspace; each component's Makefile
# offers the same ones.
.DEFAULT_GOAL := lint
include $(dir $(lastword $(MAKEFILE_LIST)))rust.mk
