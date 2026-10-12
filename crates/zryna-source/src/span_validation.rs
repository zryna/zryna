use super::{
    SourceError, SourceFile, SourceMap, Span, invalid_boundary, invalid_span_order, unknown_file,
};

impl SourceMap {
    pub(super) fn validated_source(&self, span: Span) -> Result<&SourceFile, SourceError> {
        let source = self.source(span.file).ok_or_else(|| unknown_file(span.file))?;
        if span.start > span.end {
            return Err(invalid_span_order(span.file, span.start, span.end));
        }
        let source_len = u32::try_from(source.text.len()).map_err(|_| {
            SourceError::new(
                "ZRYNA-S1007",
                Some(source.path.as_str().to_owned()),
                "source length exceeds the supported offset range",
                "reduce the source file before analysis",
            )
        })?;
        if span.end > source_len {
            return Err(SourceError::new(
                "ZRYNA-S1007",
                Some(source.path.as_str().to_owned()),
                format!(
                    "source span {}..{} exceeds file length {source_len}",
                    span.start, span.end
                ),
                "keep source ranges within the referenced file",
            ));
        }
        let start =
            usize::try_from(span.start).map_err(|_| invalid_boundary(source, span.start))?;
        let end = usize::try_from(span.end).map_err(|_| invalid_boundary(source, span.end))?;
        if !source.text.is_char_boundary(start) {
            return Err(invalid_boundary(source, span.start));
        }
        if !source.text.is_char_boundary(end) {
            return Err(invalid_boundary(source, span.end));
        }
        Ok(source)
    }
}
