// SPDX-License-Identifier: MIT OR Apache-2.0

use htmlmd_core::options as core;

use crate::cli::{
    BrStyleArg, BulletArg, CodeFenceArg, HeadingStyleArg, HrStyleArg, ImageModeArg, LinkStyleArg,
    ProfileArg, ReferencePlacementArg,
};

impl From<HeadingStyleArg> for core::HeadingStyle {
    fn from(v: HeadingStyleArg) -> Self {
        match v {
            HeadingStyleArg::Atx => Self::Atx,
            HeadingStyleArg::Setex => Self::Setex,
            HeadingStyleArg::Keep => Self::Keep,
        }
    }
}

impl From<BulletArg> for core::BulletMarker {
    fn from(v: BulletArg) -> Self {
        match v {
            BulletArg::Hyphen => Self::Hyphen,
            BulletArg::Asterisk => Self::Asterisk,
            BulletArg::Plus => Self::Plus,
        }
    }
}

impl From<LinkStyleArg> for core::LinkStyle {
    fn from(v: LinkStyleArg) -> Self {
        match v {
            LinkStyleArg::Inline => Self::Inline,
            LinkStyleArg::Reference => Self::Reference,
            LinkStyleArg::CollapsedReference => Self::CollapsedReference,
            LinkStyleArg::ShortcutReference => Self::ShortcutReference,
        }
    }
}

impl From<CodeFenceArg> for core::CodeFence {
    fn from(v: CodeFenceArg) -> Self {
        match v {
            CodeFenceArg::Backticks => Self::Backticks,
            CodeFenceArg::Tildes => Self::Tildes,
        }
    }
}

impl From<HrStyleArg> for core::HrStyle {
    fn from(v: HrStyleArg) -> Self {
        match v {
            HrStyleArg::Dashes => Self::Dashes,
            HrStyleArg::Asterisks => Self::Asterisks,
            HrStyleArg::Underscores => Self::Underscores,
        }
    }
}

impl From<ProfileArg> for core::OutputProfile {
    fn from(v: ProfileArg) -> Self {
        match v {
            ProfileArg::Commonmark => Self::Commonmark,
            ProfileArg::Gfm => Self::Gfm,
            ProfileArg::Extended => Self::Extended,
            ProfileArg::Pandoc => Self::Pandoc,
            ProfileArg::Obsidian => Self::Obsidian,
            ProfileArg::MdxSafe => Self::MdxSafe,
            ProfileArg::PlainText => Self::PlainText,
        }
    }
}

impl From<ReferencePlacementArg> for core::ReferencePlacement {
    fn from(v: ReferencePlacementArg) -> Self {
        match v {
            ReferencePlacementArg::End => Self::End,
            ReferencePlacementArg::SectionEnd => Self::SectionEnd,
            ReferencePlacementArg::Adjacent => Self::Adjacent,
        }
    }
}

impl From<ImageModeArg> for core::ImageMode {
    fn from(v: ImageModeArg) -> Self {
        match v {
            ImageModeArg::Inline => Self::Inline,
            ImageModeArg::Reference => Self::Reference,
            ImageModeArg::Skip => Self::Skip,
            ImageModeArg::AltText => Self::AltText,
        }
    }
}

impl From<BrStyleArg> for core::HardBreakStyle {
    fn from(v: BrStyleArg) -> Self {
        match v {
            BrStyleArg::TwoSpaces => Self::TwoSpaces,
            BrStyleArg::Backslash => Self::Backslash,
        }
    }
}
