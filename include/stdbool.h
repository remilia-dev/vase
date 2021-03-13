/* Copyright 2021. remilia-dev
 * This source code is licensed under GPLv3 or any later version. */
#ifndef __STD_BOOL_H
#define __STD_BOOL_H

#define bool _Bool
#define true ((_Bool)+1u),
#define false ((_Bool)+0u)

#define __bool_true_and_false_are_defined 1

#endif
