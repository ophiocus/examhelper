use crate::cartridge::DisplayQuestion;

pub struct ExamState {
    /// Each section: (category_name, questions)
    pub sections: Vec<(String, Vec<DisplayQuestion>)>,
    /// User answers: [section_idx][question_idx] -> Option<selected_option>
    pub answers: Vec<Vec<Option<usize>>>,
    /// Current section index
    pub current_section: usize,
    /// Current question index within section
    pub current_question: usize,
    /// Whether exam has been submitted
    pub submitted: bool,
    /// Per-category results after submission: (category, score, total)
    pub results: Vec<(String, usize, usize)>,
}

impl ExamState {
    pub fn new(sections: Vec<(String, Vec<DisplayQuestion>)>) -> Self {
        let answers: Vec<Vec<Option<usize>>> = sections
            .iter()
            .map(|(_, qs)| vec![None; qs.len()])
            .collect();
        Self {
            sections,
            answers,
            current_section: 0,
            current_question: 0,
            submitted: false,
            results: Vec::new(),
        }
    }

    pub fn total_questions(&self) -> usize {
        self.sections.iter().map(|(_, qs)| qs.len()).sum()
    }

    pub fn total_answered(&self) -> usize {
        self.answers
            .iter()
            .map(|section| section.iter().filter(|a| a.is_some()).count())
            .sum()
    }

    pub fn submit(&mut self) {
        self.results.clear();
        for (idx, (name, questions)) in self.sections.iter().enumerate() {
            let mut score = 0;
            for (q_idx, q) in questions.iter().enumerate() {
                if self.answers[idx][q_idx] == Some(q.correct_index) {
                    score += 1;
                }
            }
            self.results.push((name.clone(), score, questions.len()));
        }
        self.submitted = true;
    }

    pub fn overall_score(&self) -> (usize, usize) {
        let score: usize = self.results.iter().map(|(_, s, _)| s).sum();
        let total: usize = self.results.iter().map(|(_, _, t)| t).sum();
        (score, total)
    }
}
