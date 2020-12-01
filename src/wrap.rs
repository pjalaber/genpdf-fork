// SPDX-FileCopyrightText: 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: Apache-2.0 or MIT

//! Utilities for text wrapping.

use std::borrow;

use crate::style;
use crate::Context;
use crate::Mm;

#[derive(Debug)]
pub struct Fragment<'c, 's> {
    context: &'c Context,
    strings: Vec<style::StyledCow<'s>>,
    add_whitespace: bool,
    add_hyphen: bool,
}

impl<'c, 's> Fragment<'c, 's> {
    fn style(&self) -> style::Style {
        self.strings.last().map(|s| s.style).unwrap_or_default()
    }

    fn width(&self, c: char) -> usize {
        get_width(self.style().char_width(&self.context.font_cache, c))
    }
}

impl<'c, 's> textwrap::core::Fragment for Fragment<'c, 's> {
    fn width(&self) -> usize {
        let width = self
            .strings
            .iter()
            .map(|s| s.width(&self.context.font_cache))
            .sum();
        get_width(width)
    }

    fn whitespace_width(&self) -> usize {
        if self.add_whitespace {
            self.width(' ')
        } else {
            0
        }
    }

    fn penalty_width(&self) -> usize {
        if self.add_hyphen {
            self.width('-')
        } else {
            0
        }
    }
}

fn get_width(width: Mm) -> usize {
    (width.0 * 1000.0) as usize
}

pub fn prepare<'c, 's>(
    context: &'c Context,
    strings: &'s [style::StyledString],
) -> Vec<Fragment<'c, 's>> {
    // TODO: Calculate segments over string boundaries?

    let mut fragments = Vec::new();
    for s in strings {
        let words: Vec<_> = s.s.split(' ').collect();
        for (idx, word) in words.iter().enumerate() {
            let is_last_word = idx + 1 == words.len();
            let segments = split(context, word);

            for (idx, segment) in segments.iter().enumerate() {
                let is_last_segment = idx + 1 == segments.len();
                let s = style::StyledCow::new(*segment, s.style);

                fragments.push(Fragment {
                    context,
                    strings: vec![s],
                    add_whitespace: !is_last_word && is_last_segment,
                    add_hyphen: !is_last_segment,
                });
            }
        }
    }

    // Merge strings that are not separated by whitespaces or hyphens
    fragments.into_iter().fold(Vec::new(), |mut vec, fragment| {
        if let Some(last) = vec.last_mut() {
            if !last.add_whitespace && !last.add_hyphen {
                last.strings.extend(fragment.strings);
                last.add_whitespace = fragment.add_whitespace;
                last.add_hyphen = fragment.add_hyphen;
            } else {
                vec.push(fragment);
            }
        } else {
            vec.push(fragment);
        }
        vec
    })
}

#[cfg(not(feature = "hyphenation"))]
fn split<'s>(_context: &'_ Context, s: &'s str) -> Vec<&'s str> {
    vec![s]
}

#[cfg(feature = "hyphenation")]
fn split<'s>(context: &'_ Context, s: &'s str) -> Vec<&'s str> {
    use hyphenation::Hyphenator;

    let hyphenator = if let Some(hyphenator) = &context.hyphenator {
        hyphenator
    } else {
        return vec![s];
    };

    hyphenator.hyphenate(s).into_iter().segments().collect()
}

pub fn wrap<'f, 'c, 's>(
    fragments: &'f [Fragment<'c, 's>],
    width: Mm,
) -> Vec<&'f [Fragment<'c, 's>]> {
    let width = get_width(width);
    textwrap::core::wrap_fragments(fragments, |_| width)
}

pub fn finalize<'s>(line: &[Fragment<'_, 's>]) -> Vec<style::StyledCow<'s>> {
    let mut strings = Vec::new();
    for (idx, fragment) in line.iter().enumerate() {
        for s in &fragment.strings {
            strings.push(s.clone());
        }

        let suffix = if idx + 1 == line.len() {
            if fragment.add_hyphen {
                Some('-')
            } else {
                None
            }
        } else if fragment.add_whitespace {
            Some(' ')
        } else {
            None
        };

        if let Some(suffix) = suffix {
            let mut last = strings.last_mut().unwrap();
            match &mut last.s {
                borrow::Cow::Borrowed(b) => {
                    let mut s = b.to_string();
                    s.push(suffix);
                    last.s = s.into();
                }
                borrow::Cow::Owned(o) => o.push(suffix),
            }
        }
    }
    strings
}
