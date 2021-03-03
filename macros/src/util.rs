// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use syn::Attribute;

pub fn find_attribute(target: &str, attributes: &mut Vec<Attribute>) -> Option<Attribute> {
    for (i, attribute) in attributes.iter().enumerate() {
        if let Some(id) = attribute.path.get_ident() {
            if *id == target {
                return Some(attributes.remove(i));
            }
        }
    }

    None
}
