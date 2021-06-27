cmake_minimum_required(VERSION 3.10) 
project(codec VERSION 1.0)
add_executable(codec src/main.c)
set(EXECUTABLE_OUTPUT_PATH ${PROJECT_SOURCE_DIR}/target)