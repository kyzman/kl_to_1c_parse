use crate::parser::models::SectionType;

/// Состояния парсера для потоковой обработки
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserState {
    WaitingHeader,
    ReadingHeader,
    InAccountSection,
    InDocumentSection { doc_type: String },
    BetweenSections,
    EndOfFile,
}

impl ParserState {
    /// Определяет переход состояния по ключевому слову
    pub fn transition(&self, keyword: &str) -> (Self, Option<SectionType>) {
        use ParserState::*;

        match (self, keyword) {
            // Начало файла
            (WaitingHeader, "1CClientBankExchange") => (ReadingHeader, Some(SectionType::Header)),
            (WaitingHeader, _) => {
                // Игнорируем строки до заголовка
                (WaitingHeader, None)
            }

            // Открытие секций (из любого состояния кроме EndOfFile)
            (_, "СекцияРасчСчет") => {
                (InAccountSection, Some(SectionType::AccountStatement))
            }

            // СекцияДокумент с типом
            (_, k) if k.starts_with("СекцияДокумент=") => {
                let doc_type = k.strip_prefix("СекцияДокумент=").unwrap_or("").to_string();
                (
                    InDocumentSection {
                        doc_type: doc_type.clone(),
                    },
                    Some(SectionType::Document(doc_type)),
                )
            }

            // Закрытие секций
            (InAccountSection, "КонецРасчСчет") => (BetweenSections, None),
            (InDocumentSection { .. }, "КонецДокумента") => (BetweenSections, None),

            // Конец файла
            (_, "КонецФайла") => (EndOfFile, None),

            // Игнорируем неизвестные ключевые слова
            _ => (self.clone(), None),
        }
    }

    pub fn is_in_section(&self) -> bool {
        matches!(
            self,
            Self::InAccountSection | Self::InDocumentSection { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_from_waiting_header() {
        let state = ParserState::WaitingHeader;
        let (new_state, section) = state.transition("1CClientBankExchange");
        assert_eq!(new_state, ParserState::ReadingHeader);
        assert_eq!(section, Some(SectionType::Header));
    }

    #[test]
    fn test_transition_account_section() {
        let state = ParserState::ReadingHeader;
        let (new_state, section) = state.transition("СекцияРасчСчет");
        assert_eq!(new_state, ParserState::InAccountSection);
        assert_eq!(section, Some(SectionType::AccountStatement));
    }

    #[test]
    fn test_transition_document_with_type() {
        let state = ParserState::BetweenSections;
        let (new_state, section) = state.transition("СекцияДокумент=Платежное поручение");
        assert_eq!(
            new_state,
            ParserState::InDocumentSection {
                doc_type: "Платежное поручение".to_string()
            }
        );
        assert_eq!(
            section,
            Some(SectionType::Document("Платежное поручение".to_string()))
        );
    }

    #[test]
    fn test_transition_close_account() {
        let state = ParserState::InAccountSection;
        let (new_state, section) = state.transition("КонецРасчСчет");
        assert_eq!(new_state, ParserState::BetweenSections);
        assert_eq!(section, None);
    }

    #[test]
    fn test_transition_close_document() {
        let state = ParserState::InDocumentSection {
            doc_type: "Тест".to_string(),
        };
        let (new_state, section) = state.transition("КонецДокумента");
        assert_eq!(new_state, ParserState::BetweenSections);
        assert_eq!(section, None);
    }

    #[test]
    fn test_transition_end_of_file() {
        let state = ParserState::BetweenSections;
        let (new_state, section) = state.transition("КонецФайла");
        assert_eq!(new_state, ParserState::EndOfFile);
        assert_eq!(section, None);
    }

    #[test]
    fn test_transition_ignore_unknown() {
        let state = ParserState::BetweenSections;
        let (new_state, section) = state.transition("НеизвестноеКлючевоеСлово");
        assert_eq!(new_state, ParserState::BetweenSections);
        assert_eq!(section, None);
    }

    #[test]
    fn test_is_in_section() {
        assert!(!ParserState::WaitingHeader.is_in_section());
        assert!(!ParserState::ReadingHeader.is_in_section());
        assert!(ParserState::InAccountSection.is_in_section());
        assert!(
            ParserState::InDocumentSection {
                doc_type: "X".to_string()
            }
            .is_in_section()
        );
        assert!(!ParserState::BetweenSections.is_in_section());
        assert!(!ParserState::EndOfFile.is_in_section());
    }
}
