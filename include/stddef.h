/* Copyright 2021. remilia-dev
 * This source code is licensed under GPLv3 or any later version. */
#ifndef __STD_DEF_H
#define __STD_DEF_H

typedef __PTRDIFF_TYPE__ ptrdiff_t;
typedef __SIZE_TYPE__ size_t;
typedef __MAX_ALIGN_TYPE__ max_align_t;
typedef __WCHAR_TYPE__ wchar_t;

#define NULL ((void*)0)

#define offsetof(type, member) __builtin_offset_of(type, member)

#endif
