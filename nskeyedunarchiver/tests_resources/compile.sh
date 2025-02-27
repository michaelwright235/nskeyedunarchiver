#!/bin/sh
clang -Wall -framework Foundation -framework AppKit main.m -o main && ./main
