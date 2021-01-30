// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

pub struct DropTester<'a> {
    flag: &'a mut bool,
}

impl<'a> DropTester<'a> {
    pub fn new(flag: &'a mut bool) -> Self {
        DropTester { flag }
    }
}

impl Drop for DropTester<'_> {
    fn drop(&mut self) {
        *self.flag = true;
    }
}
