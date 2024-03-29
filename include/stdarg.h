/* Copyright 2021. remilia-dev
 * This source code is licensed under GPLv3 or any later version. */
#ifndef __STD_ARG_H
#define __STD_ARG_H

typedef __builtin_va_list va_list;
#define va_start(ap, param) __builtin_va_start(ap, param)
#define va_end(ap) __builtin_va_end(ap)
#define va_arg(ap, type) __builtin_va_arg(ap, type)

#endif
