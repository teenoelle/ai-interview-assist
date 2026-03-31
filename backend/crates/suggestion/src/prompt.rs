use common::messages::TranscriptSegment;

// ── Scoring helper ────────────────────────────────────────────────────────────

fn score_triggers(triggers: &[&str], q: &str) -> usize {
    triggers.iter().filter(|&&t| q.contains(t)).count()
}

// ── Trigger lists ────────────────────────────────────────────────────────────

const INTRODUCTION_TRIGGERS: &[&str] = &[
    "tell me about yourself",
    "tell us about yourself",
    "walk me through your background",
    "walk us through your background",
    "walk me through your experience",
    "walk us through your experience",
    "take me through your background",
    "take us through your background",
    "tell me about your background",
    "tell me about your experience",
    "tell us about your experience",
    "introduce yourself",
    "tell me a little about yourself",
    "tell us a little about yourself",
    "give me a brief overview",
    "give us an overview of your background",
    "share a bit about yourself",
    "share your background",
    "overview of your background",
    "overview of your experience",
];

const MOTIVATION_TRIGGERS: &[&str] = &[
    "why are you interested",
    "why do you want to work",
    "why this role",
    "why our company",
    "why this company",
    "why this position",
    "what draws you to",
    "what attracted you to",
    "why did you apply",
    "what interests you about",
    "what excites you about",
    "why do you want to join",
    "why do you want to be",
    "why do you want to leave",
    "why are you looking",
    "what made you apply",
    "what brought you to",
];

const FUTURE_TRIGGERS: &[&str] = &[
    "five years",
    "5 years",
    "see yourself in",
    "career goals",
    "where do you want to",
    "how do you see yourself growing",
    "long-term goals",
    "long term goals",
    "where do you see yourself",
    "how do you see yourself",
    "what are your long",
    "what is your long",
];

const CLOSING_TRIGGERS: &[&str] = &[
    "do you have any questions",
    "do you have questions",
    "any questions for us",
    "questions for me",
    "questions for us",
    "anything you'd like to ask",
    "anything you would like to ask",
    "is there anything you want to ask",
    "is there anything you'd like to ask",
];

const STRENGTHS_TRIGGERS: &[&str] = &[
    "what are your strengths",
    "what is your greatest strength",
    "what is your biggest strength",
    "what do you do well",
    "what makes you good at",
    "strongest skill",
    "what are you good at",
    "greatest strength",
    "biggest strength",
    "key strengths",
    "what would your friends say about you",
    "what would your colleagues say about you",
    "what would your coworkers say about you",
    "what would your manager say about you",
    "what would your teammates say about you",
    "how would your colleagues describe you",
    "how would your coworkers describe you",
    "how would your manager describe you",
    "how would your teammates describe you",
    "how would others describe you",
    "how would people describe you",
    "what sets you apart",
    "what makes you unique",
    "what do you bring to the table",
    "what value do you bring",
    "best quality",
    "top skill",
    "what do you consider your strength",
];

const BEHAVIORAL_TRIGGERS: &[&str] = &[
    "tell me about a time",
    "describe a situation",
    "give me an example of a time",
    "have you ever",
    "what was a time",
    "share an experience",
    "when have you",
    "describe when",
    "describe a time",
    "can you give me an example of",
    "give me an example where you",
    "walk me through a time",
    "walk us through a time",
    "think of a time",
    "recall a time",
];

const WEAKNESSES_TRIGGERS: &[&str] = &[
    "greatest weakness",
    "biggest weakness",
    "area for improvement",
    "what do you struggle",
    "what would your manager say you need",
    "development area",
    "where do you need to improve",
    "what would colleagues say you need to work on",
    "tell me about a weakness",
    "describe a weakness",
    "what do you find challenging professionally",
    "what do you find difficult professionally",
];

const TECHNICAL_TRIGGERS: &[&str] = &[
    "walk me through how you would design",
    "how would you architect",
    "design a system",
    "how do you debug",
    "how do you approach technical decisions",
    "how do you evaluate technology",
    "how do you stay current with",
    "walk me through your technical",
    "technical approach",
    "technology stack",
    "how do you make technical",
    "how do you choose between",
    "how do you scale",
    "how do you handle technical debt",
    "how do you ensure code quality",
    "how do you approach testing",
    "how do you approach system design",
];

const CULTURE_TRIGGERS: &[&str] = &[
    "how do you collaborate",
    "how do you work with cross-functional",
    "describe your working style",
    "how do you handle conflict",
    "what kind of work environment",
    "how do you approach feedback",
    "how do you give feedback",
    "how do you communicate with",
    "how do you handle disagreement",
    "what does collaboration look like",
    "how do you build relationships",
    "how do you manage up",
    "how do you work across teams",
    "how do you handle a difficult coworker",
    "how do you handle a difficult team member",
    "how do you handle working with someone you disagree with",
    "what is your management style",
    "how do you motivate",
    "how do you onboard",
    "how do you give and receive feedback",
];

const SITUATIONAL_TRIGGERS: &[&str] = &[
    "what would you do if",
    "how would you handle",
    "how would you approach",
    "imagine you",
    "suppose you",
    "if you were to",
    "how would you deal with",
    "hypothetically",
    "if you joined and",
    "if you discovered",
    "what would your approach be if",
    "how would you respond if",
    "what would you do when",
    "how would you prioritize",
    "if you had to",
    "let's say you",
    "picture a scenario",
    "say you were",
    "given a situation",
];

// ── Question classification ───────────────────────────────────────────────────

#[derive(Debug, Copy, Clone)]
pub enum QuestionType {
    Introduction,
    Motivation,
    Future,
    Closing,
    Strengths,
    Weaknesses,
    Behavioral,
    Situational,
    Technical,
    Culture,
    Competency,
}

/// Score-based classification returning (primary, Option<secondary>).
/// Primary = highest scoring type. Secondary = runner-up with score > 0, if any.
/// Priority order (listed first = wins ties) mirrors the frontend questionTagger.ts.
pub fn classify_question(question: &str) -> (QuestionType, Option<QuestionType>) {
    let q = question.to_lowercase();

    let candidates: &[(usize, QuestionType)] = &[
        (score_triggers(INTRODUCTION_TRIGGERS, &q), QuestionType::Introduction),
        (score_triggers(MOTIVATION_TRIGGERS,   &q), QuestionType::Motivation),
        (score_triggers(FUTURE_TRIGGERS,       &q), QuestionType::Future),
        (score_triggers(CLOSING_TRIGGERS,      &q), QuestionType::Closing),
        (score_triggers(STRENGTHS_TRIGGERS,    &q), QuestionType::Strengths),
        (score_triggers(WEAKNESSES_TRIGGERS,   &q), QuestionType::Weaknesses),
        (score_triggers(BEHAVIORAL_TRIGGERS,   &q), QuestionType::Behavioral),
        (score_triggers(SITUATIONAL_TRIGGERS,  &q), QuestionType::Situational),
        (score_triggers(TECHNICAL_TRIGGERS,    &q), QuestionType::Technical),
        (score_triggers(CULTURE_TRIGGERS,      &q), QuestionType::Culture),
    ];

    let max_score = candidates.iter().map(|(s, _)| *s).max().unwrap_or(0);
    if max_score == 0 {
        return (QuestionType::Competency, None);
    }

    let mut primary = QuestionType::Competency;
    let mut secondary: Option<QuestionType> = None;
    let mut found_primary = false;

    for &(s, qt) in candidates {
        if s == max_score && !found_primary {
            primary = qt;
            found_primary = true;
        } else if s > 0 && found_primary && secondary.is_none() {
            secondary = Some(qt);
        }
    }

    (primary, secondary)
}

/// Kept for callers that only need a behavioral check.
pub fn is_behavioral(question: &str) -> bool {
    matches!(classify_question(question).0, QuestionType::Behavioral)
}

fn question_type_topic(qt: QuestionType) -> &'static str {
    match qt {
        QuestionType::Introduction => "your career background and story",
        QuestionType::Motivation   => "why you want this role and company",
        QuestionType::Future       => "your career direction and goals",
        QuestionType::Closing      => "questions you have for the interviewer",
        QuestionType::Strengths    => "your key strengths",
        QuestionType::Weaknesses   => "an area you are actively developing",
        QuestionType::Behavioral   => "a specific past behavioral example",
        QuestionType::Situational  => "how you would handle a hypothetical situation",
        QuestionType::Technical    => "your technical approach and design thinking",
        QuestionType::Culture      => "how you collaborate and work with others",
        QuestionType::Competency   => "your professional approach and methodology",
    }
}

pub fn question_type_to_tag(qt: QuestionType) -> &'static str {
    match qt {
        QuestionType::Introduction => "personal",
        QuestionType::Motivation   => "motivation",
        QuestionType::Future       => "future",
        QuestionType::Closing      => "closing",
        QuestionType::Strengths    => "strengths",
        QuestionType::Weaknesses   => "weaknesses",
        QuestionType::Behavioral   => "behavioral",
        QuestionType::Situational  => "situational",
        QuestionType::Technical    => "technical",
        QuestionType::Culture      => "culture",
        QuestionType::Competency   => "general",
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

fn make_ctx_prefix(transcript: &[TranscriptSegment]) -> String {
    let recent: Vec<&TranscriptSegment> = transcript.iter().rev().take(10).collect();
    let context = recent.iter().rev().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ... ");
    if context.is_empty() { String::new() } else { format!("Recent conversation: {}\n\n", context) }
}

pub fn build_user_prompt(question: &str, transcript: &[TranscriptSegment]) -> String {
    let (qtype, _) = classify_question(question);
    tracing::info!("Question type: {:?} — {:?}", qtype, question);
    let ctx_prefix = make_ctx_prefix(transcript);
    dispatch_prompt(&ctx_prefix, question, qtype)
}

pub fn build_user_prompt_for_type(question: &str, transcript: &[TranscriptSegment], qtype: QuestionType) -> String {
    let ctx_prefix = make_ctx_prefix(transcript);
    dispatch_prompt(&ctx_prefix, question, qtype)
}

pub fn build_compound_user_prompt(question: &str, transcript: &[TranscriptSegment], primary: QuestionType, secondary: QuestionType) -> String {
    let ctx_prefix = make_ctx_prefix(transcript);
    build_compound_prompt(&ctx_prefix, question, primary, secondary)
}

fn dispatch_prompt(ctx_prefix: &str, question: &str, qtype: QuestionType) -> String {
    match qtype {
        QuestionType::Introduction => build_introduction_prompt(ctx_prefix, question),
        QuestionType::Motivation   => build_motivation_prompt(ctx_prefix, question),
        QuestionType::Future       => build_future_prompt(ctx_prefix, question),
        QuestionType::Closing      => build_closing_prompt(ctx_prefix, question),
        QuestionType::Strengths    => build_strengths_prompt(ctx_prefix, question),
        QuestionType::Weaknesses   => build_weaknesses_prompt(ctx_prefix, question),
        QuestionType::Behavioral   => build_behavioral_prompt(ctx_prefix, question),
        QuestionType::Situational  => build_situational_prompt(ctx_prefix, question),
        QuestionType::Technical    => build_technical_prompt(ctx_prefix, question),
        QuestionType::Culture      => build_culture_prompt(ctx_prefix, question),
        QuestionType::Competency   => build_competency_prompt(ctx_prefix, question),
    }
}

// ── Template builders ─────────────────────────────────────────────────────────

fn build_introduction_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked an introduction question: '{}'\n\n\
CRITICAL: This is an INTRODUCTION question. Use ONLY the labeled fields below.\n\
DO NOT output Acknowledge:, Solve:, Bridge:, Present:, Past:, Future:, or Answer: — those labels do not exist here.\n\n\
Summary: <2 sentences. Draw from ALL candidate context — CV, LinkedIn About/Summary, and Early Career & Additional Context notes — to build the broadest accurate picture of the career span. Sentence 1: broad aggregate span — 'I\\'ve spent my career in [role types] across [industries/domains].' or 'My career has spanned [role types] across [industries/domains].' Names ALL the types of roles and industries across the full career, not just the most recent. Sentence 2: the narrowing focus — 'My focus has always been on [specific area].' or 'But I\\'ve mainly focused on [specific area].' or 'With a particular focus on [specific skill or domain].' Max 10 words per sentence. Speak as naturally said aloud — not a resume bullet.>\n\
Thread: <1 sentence. The connecting quality or role the candidate plays that runs through all those experiences AND makes them the natural fit for this employer's specific challenge (from the system prompt). Do NOT just name a skill — name the pattern of impact or the role they play for others (e.g. 'being the person teams rely on to turn ambiguous briefs into measurable results'). Starts with 'The throughline of my career has been', 'Throughout all of that,', 'What connects all of it is', or 'At the heart of all of it is'. Must land as relevant to what this employer needs — not just a personal brand statement. Max 20 words.>\n\
Transition1: <1 sentence bridging Thread to Story. Starts with 'Let me walk you through how I got here.' or 'That started with' or 'Earlier in my career,'. Max 10 words.>\n\
Story: <2-3 sentences. Each starts with 'I'. Insight-driven past moves — the formative experiences that shaped the candidate. Draw from ALL candidate context: CV job entries, LinkedIn About/Summary section, and Early Career & Additional Context notes. These may contain early roles that predate the CV. Use framing like: 'That role showed me I was passionate about [X].' or 'After that I learned it was important to be at a company that values [X].' or 'That was a foundational experience when it comes to [X].' No invented details.>\n\
Transition2: <1 sentence bridging Story to Next. Starts with 'Which is why,' or 'That path led me to' or 'Building on that,'. Max 10 words.>\n\
Next: <1-2 sentences. Each starts with 'I'. Forward momentum: what you are currently focused on or looking to do next. Use framing like: 'As the next step I\\'m looking to...' or 'What I\\'m currently focused on is...' or 'Looking ahead I\\'m focused on...'. Names the specific direction — not vague aspiration.>\n\
Transition3: <1 sentence bridging Next to Close. Starts with 'So when I came across' or 'That context is exactly why' or 'Which is what led me here'. Max 10 words.>\n\
Close: <2 sentences. First: connect your Next to why this specific employer and challenge appeals, referencing the employer\\'s problem from the system prompt. Starts with 'Given that I\\'m focused on' or 'Since my next step is about' or 'As I look to [next step],'. Second: enthusiasm bridge — exactly 'And that\\'s why I\\'m so excited to be speaking with you today.' Max 35 words total. Never say \\'this role\\', \\'this\\', \\'it\\' as vague pronouns.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you\\'re asking about — e.g. 'team structure', 'client mix', 'success metrics'> | <Specific grammatical question the candidate asks the interviewer. Directly related to the type of work or challenge described by the interviewer. Ends with '?'.> | <1 sentence if asked 'why do you ask?' — starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase> | <A different specific question about the opportunity. Names the concrete topic — no vague pronouns. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY: Summary:, Thread:, Transition1:, Story:, Transition2:, Next:, Transition3:, Close:, then two Ask: lines. No other labels. No preamble.\n\
- Summary and Story draw ONLY from candidate background — no invented details.\n\
- Thread must bridge background to role: it names the pattern that makes the candidate the natural fit for this employer's challenge — not just what they've done. If the background shows someone who is trusted to make things work across teams or clients, Thread should name that role (e.g. 'being the go-to person who turns complexity into outcomes'). Always connect Thread to the employer's challenge from the system prompt.\n\
- Next names the specific direction the candidate is building toward — not generic praise.\n\
- Close first sentence must reference the employer\\'s specific challenge from the system prompt AND connect to Next.\n\
- Close second sentence must be exactly: 'And that\\'s why I\\'m so excited to be speaking with you today.'\n\
- Ask topics: short noun phrases (2-4 words), not verb phrases. Directly related to what the interviewer asked about.\n\
- NEVER invent metrics, percentages, dollar figures, headcount, or timeframes.\n\
- NEVER name specific companies, clients, or employers — refer by industry only.\n\
- No adjectives or adverbs. No 'passionate', 'excited', 'dedicated', 'driven'. Facts and direction only.",
        ctx_prefix, question
    )
}

fn build_motivation_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a motivation question: '{}'\n\n\
CRITICAL: This is a MOTIVATION question. Output ONLY the five labeled lines below.\n\
DO NOT output Acknowledge:, Solve:, Bridge:, or Answer: — those labels do not exist here.\n\n\
Company: <1-2 sentences. Each starts with 'I'. Max 10 words each. The employer's specific business challenge or mission from the system prompt — name the actual problem, not generic praise.>\n\
Transition1: <1 sentence connecting Company to Role. Starts with 'That challenge maps directly to' or 'Which is where' or 'My background fits because'. Max 10 words.>\n\
Role: <2 sentences. Each starts with 'I'. Max 10 words each. How your specific background maps to this role's requirements. Draw only from background provided. No invented details.>\n\
Transition2: <1 sentence connecting Role to Self. Starts with 'But beyond the skillset,' or 'On a personal level,' or 'And what draws me further is'. Max 10 words.>\n\
Self: <1-2 sentences. Each starts with 'I'. Max 10 words each. How this role fits your career trajectory. What you will build or develop here that you cannot elsewhere.>\n\
Transition3: <1 sentence connecting Self to Close. Starts with 'So when I look at' or 'That combination is exactly why' or 'Which is why'. Max 10 words.>\n\
Close: <One sentence. Connects your motivation to the employer's specific challenge from the system prompt. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn from the specific topic the interviewer raised. e.g. 'business challenge', 'client mix', 'team priorities'> | <Specific grammatical question probing an aspect of what the interviewer asked about. Names a concrete challenge, outcome, or constraint from the system prompt. Ends with '?'.> | <1 sentence if asked 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle from the first, still related to the interviewer's question> | <A different specific question. Names a concrete metric, process, or domain — no vague pronouns. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY: Company:, Transition1:, Role:, Transition2:, Self:, Transition3:, Close:, then two Ask: lines. No other labels. No preamble.\n\
- Company names the employer's actual challenge from the system prompt. Never 'great company' or 'exciting opportunity'.\n\
- Role draws ONLY from candidate background — no invented details.\n\
- NEVER invent metrics, percentages, dollar figures, headcount, or timeframes. If no specific figure exists in the candidate background, use directional language only (e.g. 'improved', 'reduced', 'grew') — never fabricate a number.\n\
- NEVER name specific companies, clients, or employers — refer by industry only (e.g. 'a retail brand', 'a tech startup').\n\
- Ask topics: 2-4 word noun phrases naming the specific thing being asked about — e.g. 'team structure', 'success metrics', 'client mix'. Must directly relate to what the interviewer asked. Never a verb phrase. Never vague.\n\
- Ask questions must probe the specific topic the interviewer raised — not generic role questions. If the recent conversation includes specific words or concerns the interviewer mentioned, prioritise those for Ask topics.\n\
- No adjectives or adverbs. No 'passionate', 'excited', 'dedicated', 'driven'. Facts and direction only.",
        ctx_prefix, question
    )
}

fn build_future_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a future/growth question: '{}'\n\n\
CRITICAL: This is a FUTURE/GROWTH question. Output ONLY the five labeled lines below.\n\
DO NOT output Acknowledge:, Solve:, Bridge:, or Answer: — those labels do not exist here.\n\n\
Direction: <1-2 sentences. Each starts with 'I'. Max 10 words each. Where you are professionally headed. Names the specific intersection of skills or domain you are building toward. Never 'dream job' or 'passionate'.>\n\
Transition1: <1 sentence connecting Direction to Alignment. Starts with 'That path leads directly to' or 'Which is why this employer' or 'And this role sits on that path because'. Max 10 words.>\n\
Alignment: <1-2 sentences. Each starts with 'I'. Max 10 words each. How this specific role and employer sit directly on that path. Names the employer's challenge or growth area from the system prompt.>\n\
Transition2: <1 sentence connecting Alignment to Contribution. Starts with 'Concretely,' or 'In practical terms,' or 'What I would bring here is'. Max 10 words.>\n\
Contribution: <1 sentence. Starts with 'I'. Max 10 words. What you will build or deliver here along the way. Concrete outcome, not vague aspiration.>\n\
Transition3: <1 sentence connecting Contribution to Close. Starts with 'Taken together,' or 'That is the reason' or 'So in short,'. Max 10 words.>\n\
Close: <One sentence. Connects your trajectory to the employer's specific challenge from the system prompt. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn from the specific topic the interviewer raised. e.g. 'growth path', 'skill development', 'team challenge'> | <Specific grammatical question probing an aspect of what the interviewer asked about. Names a concrete skill, domain, or outcome. Ends with '?'.> | <1 sentence if asked 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle, still related to the interviewer's question> | <A different specific question about the opportunity or challenge ahead. Names a concrete metric, process, or domain. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY: Direction:, Transition1:, Alignment:, Transition2:, Contribution:, Transition3:, Close:, then two Ask: lines. No other labels. No preamble.\n\
- Direction: specific but not rigid. Never 'dream' or 'passionate'. Facts and direction only.\n\
- Alignment names the employer's actual challenge from the system prompt.\n\
- NEVER invent metrics, percentages, dollar figures, headcount, or timeframes. If no specific figure exists in the candidate background, use directional language only (e.g. 'improved', 'reduced', 'grew') — never fabricate a number.\n\
- NEVER name specific companies, clients, or employers — refer by industry only (e.g. 'a retail brand', 'a tech startup').\n\
- Ask topics: 2-4 word noun phrases naming the specific thing being asked about. Must directly relate to what the interviewer asked — not generic role questions. If the recent conversation includes specific words or concerns the interviewer mentioned, prioritise those for Ask topics.\n\
- No adjectives or adverbs. No 'passionate', 'excited', 'dedicated', 'driven'. Facts and direction only.",
        ctx_prefix, question
    )
}

fn build_closing_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a closing question: '{}'\n\n\
CRITICAL: This is a CLOSING question — the interviewer is asking if you have questions.\n\
DO NOT output Acknowledge:, Solve:, Bridge:, Answer:, or Close: — those labels do not exist here.\n\
Output ONLY the --- separator followed by exactly 4 Ask: lines.\n\n\
---\n\
Ask: <2-4 word noun phrase — the employer's core challenge, drawn from the system prompt. e.g. 'revenue growth constraint', 'client retention challenge'> | <Question probing the employer's specific business challenge. Names the exact problem from the system prompt. Ends with '?'.> | <1 sentence if asked 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — how success is defined in this role> | <Question about how success is measured — names the specific outcome, metric, or deliverable. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — team structure or way of working> | <Question about how the team works or is structured around the key challenge. Names the specific process, tool, or domain. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — company or team direction> | <Forward-looking question about where the company or team is headed. Names the specific domain or growth area. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY the --- separator and then exactly 4 Ask: lines. No other content.\n\
- All 4 Ask: lines are questions the CANDIDATE asks the INTERVIEWER.\n\
- Ask topics: 2-4 word noun phrases naming the specific thing being asked about. Never verb phrases. Never vague.\n\
- Every question names a specific metric, process, tool, or domain from the system prompt — never 'this', 'it', or vague pronouns.\n\
- Draw on the employer's challenge from the system prompt — not generic interview questions.\n\
- Never repeat a topic across Ask lines.\n\
- NEVER name specific clients or companies. Refer by industry only.",
        ctx_prefix, question
    )
}

fn build_strengths_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked: '{}'\n\n\
IMPORTANT: Output ONLY the exact labeled lines below. No preamble, no intro, nothing extra.\n\n\
Acknowledge: <One complete grammatical sentence the candidate speaks aloud. Acknowledges the business priority behind asking about strengths — connects to the employer's specific challenge from the system prompt. e.g. 'It sounds like this role requires strong [relevant skill] to deliver [employer's specific challenge].' Max 20 words. Never starts with 'I'. Never restates the question.>\n\
Answer: <Two to three strengths, each as a [keyword] strategy block. Each strength MUST begin with a [1-2 word keyword] immediately before its opening sentence — no space between ] and first word.\n\
For each strength:\n\
(A) [keyword] + one sentence naming the strength and its primary business outcome. Direct claim — not 'I am good at'. e.g. '[data analysis] Identifying the right metric separates high-impact campaigns from wasted spend.'\n\
(B) One sentence of evidence from the candidate background — brief proof point. e.g. 'At a [industry] client, I rebuilt attribution tracking which revealed a 40% budget misallocation.' Draw only from background provided. NEVER invent metrics — use directional language if no number exists.\n\
(C) One sentence connecting this strength to the employer's specific challenge from the system prompt. e.g. 'For [employer's challenge], this means [directional outcome].'\n\
Strength 2 onward: opening sentence starts with 'Beyond that,' or 'I also bring'.\n\
Last sentence: names the combined business value these strengths bring to this specific role.\n\
No adjectives. No 'I am passionate'. No 'utilize'. Facts and outcomes only.>\n\
Close: <One sentence connecting your strengths to the employer's specific business challenge. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. Never say 'this role', 'this work', 'this', 'it', or 'that'.>\n\
---\n\
Example: [1-2 word keyword] 3-5 word outcome title | <Concrete proof point. 3-4 sentences max. All on ONE line. Each starts with 'I'. Situation + Action combined in first sentence. Result in last sentence. Draw only from background. No invented metrics.>\n\
Ask: <2-4 word noun phrase — directly related to what the interviewer asked about strengths. e.g. 'highest impact area', 'skill application', 'team gap'> | <Question probing where the specific strengths just discussed would have the most impact — names the domain, metric, or challenge from the system prompt. Ends with '?'.> | <1 sentence if asked 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle on what the interviewer raised> | <A different question about what the team most needs — names the specific skill area or outcome. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one sentence naming the business priority behind the question. Never starts with 'I'.\n\
- Answer text must be on the same line as 'Answer:' — not on a new line.\n\
- Answer: each strength MUST begin with [keyword] immediately before the outcome sentence. No space between ] and first word.\n\
- Answer: strength evidence draws ONLY from candidate background. NEVER invent metrics — use directional language only if no figure exists in the background.\n\
- Answer: strength connection names the employer's actual challenge from the system prompt.\n\
- Answer: 2-3 strengths only. Never more than 3.\n\
- Close: one sentence. Max 20 words. Never say 'this role'.\n\
- Always use 'I' — never 'we' or 'our'.\n\
- Acronyms: write in full on first use followed by abbreviation in parentheses.\n\
- NEVER name specific clients or companies. Refer by industry only (e.g. 'retail brand', 'tech startup').\n\
- Ask topics: 2-4 word noun phrases naming the specific thing being asked about. Must directly relate to what the interviewer asked — not generic role questions. If the recent conversation includes specific words or concerns the interviewer mentioned, prioritise those.\n\
- Use only background provided. No invented details.",
        ctx_prefix, question
    )
}

fn build_behavioral_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a behavioral question: '{}'\n\n\
IMPORTANT: Output ONLY the exact labeled lines below. No preamble, no intro, nothing extra.\n\n\
Acknowledge: <One complete grammatical sentence the candidate speaks aloud to the interviewer. Acknowledge the business concern or priority behind the question — do NOT restate the question. Open with one of: 'It sounds like the company is focused on', 'It seems like the priority here is', 'I can see this role is important for', 'It sounds like the team is working through', 'From your question, I can see the focus is on'. Then complete the sentence with the specific business outcome or concern. Max 20 words. Never say 'our'. End with a period.>\n\
Solve: <One sentence spoken after Acknowledge. Shows the candidate has direct experience solving this exact type of business problem. Starts with 'I' or 'I\'ve'. Max 12 words. Must name the specific business outcome from Acknowledge — never vague pronouns. e.g. 'I\'ve built paid search systems that directly reduce customer acquisition cost.' or 'I\'ve led attribution strategy across high-spend performance accounts.'>\n\
Bridge: <One short spoken sentence that transitions from Solve to the Answer. 5-8 words. Starts with 'I\'d' or 'I'. e.g. 'I\'d approach that by starting with the data.' or 'I\'d tackle that by auditing the targeting first.' Never a question. Never starts with 'We' or 'Here\'s'.>\n\
Answer: <The spoken STAR story on this same line. Short sentences starting with 'I'. Max 10 words per sentence. Structure: (1) Situation — one sentence of brief context: 'In [brief context], I [role or task].' (2) Action 1 — leads with a specific verb: 'I [action verb] [specific approach] to [business outcome].' Never 'I address'. (3) REQUIRED inline illustration for action 1 — 'So if [specific trigger], I [specific action], which would [directional outcome].' e.g. 'So if a client\'s CPA rises, I audit targeting and form messaging, which would bring conversion costs down.' (4) If a second action applies: soft connector — 'I also [action].' or 'Beyond that, I [action].' then REQUIRED inline illustration. (5) Result — last sentence names the directional outcome achieved. No upfront listing. No adjectives. No adverbs. No 'utilize'. Never use vague pronouns — name metrics, channels, tools, and processes explicitly.>\n\
Close: <One sentence the candidate says after the Answer. Mirrors the employer\'s specific business challenge from the system prompt — name the exact problem the employer is trying to solve (their growth constraint, market challenge, or operational goal) not a generic domain. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. End with a period. Never say 'this role', 'this work', 'this', 'it', or 'that'.>\n\
---\n\
Example: [Example] <keyword phrase: outcome. Include a metric ONLY if it appears explicitly in the candidate background — never invent one. Max 6 words total. e.g. 'difficult client: retained account'>\n\
Pivot: [Pivot] <Only include if the candidate background has NO direct example matching this behavioral question. Identify the closest transferable experience from the background. Format: '[transferable skill or context] → [how it applies to the question]'. Max 8 words. If the background has a direct example, omit this line entirely.>\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn directly from the topic the interviewer raised. e.g. 'team prioritization', 'success metrics', 'client feedback loop'> | <A genuine question the candidate asks the interviewer. Names a specific metric, tool, process, or concept related to what the interviewer asked about. Ends with '?'.> | <1 sentence if asked 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle, still related to the interviewer's question> | <A different genuine question. Names the specific topic — no vague pronouns. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one complete sentence the candidate reads aloud. Empathetic and conversational — names the business priority without restating the question. Never starts with 'I'.\n\
- Solve: one sentence starting with 'I'. Names the specific business outcome from Acknowledge. Max 12 words. No vague pronouns.\n\
- Bridge: one short spoken sentence, 5-8 words, that transitions from Solve to the Answer. Starts with 'I\'d' or 'I'. Never a question. Never starts with 'We' or 'Here\'s'.\n\
- Answer text must be on the same line as 'Answer:' — not on a new line.\n\
- Answer (STAR): first sentence sets context — 'In [brief situation], I [task].' Each subsequent sentence leads with a specific action verb. Never 'I address'. No upfront listing.\n\
- Answer: each action is introduced one at a time. Action 2 onward uses a soft connector: 'I also [action].' or 'Beyond that, I [action].'\n\
- Answer: EVERY action claim MUST be immediately followed by one inline illustration — no exceptions. Pattern: 'So if [specific trigger], I [specific action], which would [directional outcome].' Directional language only — never a percentage or fabricated number.\n\
- Answer: last sentence names the directional outcome achieved.\n\
- Answer: no adjectives or adverbs. No 'utilize'. No arrows (→). Write all sentences in full. Facts and actions only.\n\
- Pivot: only include if the background has NO direct example for this behavioral question. Identifies the closest transferable experience from the background. Omit entirely if a direct example exists.\n\
- Close: one sentence mirroring the employer\'s specific business challenge from the system prompt. Name the exact problem the employer is solving — not a generic domain or skill. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. Never say 'this role', 'this work', 'this', 'it', or 'that'.\n\
- Always use 'I' — never 'we', 'our team', or 'we found'. The candidate speaks only about their own actions and decisions.\n\
- Acronyms: always write in full on first use followed by the abbreviation in parentheses — e.g. 'Customer Acquisition Cost (CAC)', 'Return on Ad Spend (ROAS)', 'Search Engine Optimization (SEO)'. Never use a bare acronym without first defining it.\n\
- NEVER invent metrics, percentages, or numbers. Only use figures explicitly stated in the candidate background. If an outcome improved but the magnitude is unknown, describe the direction only (e.g. 'improved conversion') — never fabricate a number.\n\
- General and Example hints: max 6 words total (keyword phrase + 2-3 word result/approach).\n\
- Only add a second General or Example if it addresses a genuinely DIFFERENT part of the question or a different story. No repeating the same point in different words.\n\
- Keywords are multi-word phrases from the question (e.g. 'difficult conversation', 'conflicting priorities', 'client relationships').\n\
- Ask topics: 2-4 word noun phrases naming the specific thing being asked about. Must directly relate to the topic the interviewer raised — not generic role questions. If the recent conversation includes specific words or concerns the interviewer mentioned, prioritise those for Ask topics. Never a verb phrase.\n\
- Ask question: natural, specific, grammatical question the candidate asks the interviewer. Ends with '?'. No adjectives or adverbs. Never use 'this', 'it', 'that', or vague pronouns — always name the specific metric, tool, process, or concept explicitly.\n\
- Ask follow-up (3rd pipe segment): REQUIRED on every Ask line. 1 sentence the candidate says if the interviewer asks 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words. Must appear after the second pipe — never omit it.\n\
- Ask lines come AFTER the --- separator only.\n\
- NEVER name specific clients, employers, or companies. Refer to them by industry only (e.g. 'retail brand', 'tech startup', 'financial services firm').\n\
- Read the system prompt carefully to understand the employer's business model. If the employer is an agency, consultancy, or services firm that works with multiple clients, frame all answers in terms of client work across accounts — NEVER describe it as owning one company's strategy long-term.\n\
- Use only background provided. No invented details.",
        ctx_prefix, question
    )
}

fn build_competency_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked: '{}'\n\n\
IMPORTANT: Output ONLY the exact labeled lines below. No preamble, no intro, nothing extra.\n\n\
Acknowledge: <One complete grammatical sentence the candidate speaks aloud to the interviewer. Acknowledge the business concern or priority behind the question — do NOT restate the question. Open with one of: 'It sounds like the company is focused on', 'It seems like the priority here is', 'I can see this role is important for', 'It sounds like the team is working through', 'From your question, I can see the focus is on'. Then complete the sentence with the specific business outcome or concern. Max 20 words. Never say 'our'. End with a period.>\n\
Solve: <One sentence spoken after Acknowledge. Shows the candidate has direct experience solving this exact type of business problem. Starts with 'I' or 'I\'ve'. Max 12 words. Must name the specific business outcome from Acknowledge — never vague pronouns. e.g. 'I\'ve built paid search systems that directly reduce customer acquisition cost.' or 'I\'ve led attribution strategy across high-spend performance accounts.'>\n\
Bridge: <One short spoken sentence that transitions from Solve to the Answer. 5-8 words. Starts with 'I\'d' or 'I'. e.g. 'I\'d approach that by starting with the data.' or 'I\'d tackle that by auditing the targeting first.' Never a question. Never starts with 'We' or 'Here\'s'.>\n\
Answer: <The spoken answer on this same line. Short sentences. Max 10 words per sentence. Each strategy MUST begin with a [1-2 word keyword] immediately before its outcome sentence — no space between ] and the first word of the outcome. e.g. '[targeting] Customer acquisition cost rises when targeting drifts. I audit targeting and form messaging because drift is the most common cost driver. So if a client\'s CPA rises, I audit targeting and form changes, which would bring conversion costs down.[attribution] Beyond that, attribution gaps hide which channels drive revenue. I implement multi-touch attribution because single-touch models misallocate budget. So if ROAS drops, I audit the attribution model, which would reveal true revenue drivers.' Each strategy follows this 3-part pattern: (A) [keyword] + outcome sentence. (B) 'I [action verb] [specific approach] because [why it addresses the outcome].' (C) REQUIRED illustration — 'So if [specific trigger], I [specific action], which would [directional outcome].' Strategy 2 onward: outcome sentence opens with 'Beyond that,' or 'I also find that'. Last sentence names the overall business impact. No upfront listing. No adjectives. No adverbs. No 'utilize'. Never use vague pronouns.>\n\
Close: <One sentence the candidate says after the Answer. Mirrors the employer\'s specific business challenge from the system prompt — name the exact problem the employer is trying to solve (their growth constraint, market challenge, or operational goal) not a generic domain. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. End with a period. Never say 'this role', 'this work', 'this', 'it', or 'that'.>\n\
---\n\
Example: [1-2 word keyword] 3-5 word outcome title | <STAR story. 4 sentences maximum. All on ONE line. Each sentence starts with 'I'. Max 10 words per sentence. (1) Situation + Action combined — 'In [brief context], I [action verb] [specific approach].' (2) Optional second action — 'I also [action verb] [approach].' (3) Result — last sentence names the directional outcome achieved. NO inline 'So if' illustration. Draw only from candidate background. No invented metrics. Never use vague pronouns.>\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn directly from the topic the interviewer raised. e.g. 'attribution model', 'team prioritization', 'data maturity'> | <A genuine question the candidate asks the interviewer. Names a specific metric, tool, process, or concept related to what the interviewer asked about. Ends with '?'.> | <1 sentence if asked 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle, still related to the interviewer's question> | <A different genuine question. Names the specific topic — no vague pronouns. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one complete sentence the candidate reads aloud. Empathetic and conversational — names the business priority without restating the question. Never starts with 'I'.\n\
- Solve: one sentence starting with 'I'. Names the specific business outcome from Acknowledge. Max 12 words. No vague pronouns.\n\
- Answer text must be on the same line as 'Answer:' — not on a new line.\n\
- Answer: each strategy MUST begin with [1-2 word keyword] immediately before the outcome sentence. No space between ] and first word. e.g. '[targeting] Customer acquisition cost rises...'\n\
- Answer: the FIRST strategy's outcome sentence is direct. NEVER start with 'Beyond that', 'I also', 'On top of that' — those are only for strategy 2 onward.\n\
- Answer: each strategy follows this exact 3-step sequence — step 1: [keyword] + outcome sentence; step 2: 'I [action] [approach] because [why it works]'; step 3: 'So if [trigger], I [action], which would [directional outcome].' DO NOT skip step 3 under any circumstances.\n\
- Answer: strategy 2 onward: outcome sentence opens with 'Beyond that,' or 'I also find that'. Then step 2 and step 3 follow identically.\n\
- Answer: every strategy MUST complete all 3 steps before moving to the next. Directional language only — never a percentage or fabricated number.\n\
- Answer: last sentence names the overall business impact achieved.\n\
- Answer: no adjectives or adverbs. No 'utilize'. No arrows (→). Write all sentences in full. Facts and actions only.\n\
- Bridge: one short spoken sentence, 5-8 words, that transitions from Solve to the Answer. Starts with 'I\'d' or 'I'. Never a question. Never starts with 'We' or 'Here\'s'.\n\
- Close: one sentence mirroring the employer\'s specific business challenge from the system prompt. Name the exact problem the employer is solving — not a generic domain or skill. Starts with 'That\'s why', 'This is why', or 'I\'m confident'. Max 20 words. Never say 'this role', 'this work', 'this', 'it', or 'that'.\n\
- Always use 'I' — never 'we', 'our team', or 'we found'. The candidate speaks only about their own actions and decisions.\n\
- Acronyms: always write in full on first use followed by the abbreviation in parentheses — e.g. 'Customer Acquisition Cost (CAC)', 'Return on Ad Spend (ROAS)', 'Search Engine Optimization (SEO)'. Never use a bare acronym without first defining it.\n\
- NEVER invent metrics, percentages, or numbers. Only use figures explicitly stated in the candidate background. If an outcome improved but the magnitude is unknown, describe the direction only (e.g. 'improved conversion') — never fabricate a number.\n\
- Example: [keyword] outcome title | STAR story. 4 sentences maximum. No inline 'So if' illustration. Situation embedded as a clause in the first Action sentence. Draw only from candidate background. No invented metrics.\n\
- Keywords are multi-word phrases from the question (e.g. 'keyword research', 'client relationships', 'ad copywriting').\n\
- Ask topics: 2-4 word noun phrases naming the specific thing being asked about. Must directly relate to the topic the interviewer raised — not generic role questions. If the recent conversation includes specific words or concerns the interviewer mentioned, prioritise those for Ask topics. Never a verb phrase.\n\
- Ask question: natural, specific, grammatical question the candidate asks the interviewer. Ends with '?'. No adjectives or adverbs. Never use 'this', 'it', 'that', or vague pronouns — always name the specific metric, tool, process, or concept explicitly.\n\
- Ask follow-up (3rd pipe segment): REQUIRED on every Ask line. 1 sentence the candidate says if the interviewer asks 'why do you ask?'. Starts with 'I ask because' or 'I'm curious about'. Max 15 words. Must appear after the second pipe — never omit it.\n\
- Ask lines come AFTER the --- separator only.\n\
- NEVER name specific clients, employers, or companies. Refer to them by industry only (e.g. 'retail brand', 'tech startup', 'financial services firm').\n\
- Use only background provided. No invented details.",
        ctx_prefix, question
    )
}

fn build_technical_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a technical question: '{}'\n\n\
CRITICAL: This is a TECHNICAL question. Output ONLY the exact labeled lines below. No preamble.\n\n\
Acknowledge: <One complete sentence naming the technical challenge or business stake behind the question — not the question itself. Opens with: 'It sounds like the challenge here is', 'From your question, the concern seems to be', or 'I can see the priority is'. Completes with the specific technical problem or constraint, drawn from the job description and company context. Max 20 words. Never starts with 'I'.>\n\
Solve: <One sentence. Names the candidate's specific technical background most directly relevant to this question. Starts with 'I' or 'I\\'ve'. Max 12 words. Draws from candidate background in the system prompt — CV roles, LinkedIn, portfolio, extra experience notes. No invented details.>\n\
Bridge: <One short sentence transitioning from Solve to the Answer. 5-8 words. Starts with 'I\\'d' or 'I'. Never a question. Never starts with 'We' or 'Here\\'s'.>\n\
Answer: <Technical reasoning on this same line. Each strategy MUST begin with a [1-2 word keyword] immediately before its opening sentence — no space between ] and first word. For each strategy: (A) [keyword] + one outcome sentence naming the technical principle or decision. (B) 'I [action verb] [specific approach] because [technical reason].' (C) REQUIRED illustration — 'So if [specific technical scenario], I [specific action], which would [directional outcome].' Strategy 2 onward: outcome sentence opens with 'Beyond that,' or 'I also'. Last sentence names the overall technical or business outcome. 2-3 strategies. No adjectives. No invented metrics. Draws from candidate background.>\n\
Close: <One sentence connecting the candidate\\'s technical approach to the employer\\'s specific challenge from the system prompt. Starts with 'That\\'s why', 'This is why', or 'I\\'m confident'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Example: [1-2 word keyword] 3-5 word outcome title | <STAR story. 4 sentences max. All on ONE line. Each starts with 'I'. Situation + Action in first sentence. Technical outcome in last sentence. Draws only from candidate background. No invented metrics.>\n\
Ask: <2-4 word noun phrase — the specific technical challenge or system named in the question> | <Question probing the technical depth of the problem — names the specific system, constraint, or scale involved. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different technical angle> | <A different question about tooling, architecture decisions, or technical tradeoffs the team faces. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one sentence naming the technical stake. Never starts with 'I'. Draws from system prompt context.\n\
- Solve: one sentence starting with 'I'. Names specific technical experience. Max 12 words. No vague pronouns.\n\
- Bridge: 5-8 words. Starts with 'I\\'d' or 'I'. Never a question.\n\
- Answer text must be on the same line as 'Answer:'.\n\
- Answer: [keyword] strategy format. Each strategy completes all 3 steps. Directional language only — never fabricate metrics.\n\
- Close: one sentence. Max 20 words. References employer\\'s specific challenge. Never say 'this role'.\n\
- Always use 'I' — never 'we' or 'our'.\n\
- Acronyms: write in full on first use followed by abbreviation in parentheses.\n\
- NEVER invent metrics, percentages, dollar figures, or timeframes not in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Use only background provided. No invented details.",
        ctx_prefix, question
    )
}

fn build_culture_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a culture/collaboration question: '{}'\n\n\
CRITICAL: This is a CULTURE question about working style and collaboration. Output ONLY the exact labeled lines below. No preamble.\n\n\
Acknowledge: <One complete sentence naming the team dynamic or collaboration challenge behind the question — not the question itself. Opens with: 'It sounds like the team values', 'From your question, the priority seems to be', or 'I can see the focus here is on'. Completes with the specific collaboration need or team challenge drawn from the job description and company context. Max 20 words. Never starts with 'I'.>\n\
Solve: <One sentence naming the candidate\\'s working style or approach that directly addresses the collaboration need from Acknowledge. Starts with 'I' or 'I\\'ve'. Max 12 words. Draws from candidate background in the system prompt. No invented details.>\n\
Bridge: <One short sentence transitioning to the Answer. 5-8 words. Starts with 'I\\'d' or 'I'. Never a question.>\n\
Answer: <Behavioral story on this same line. Short sentences starting with 'I'. Max 10 words per sentence. Structure: (1) Context — one sentence: 'In [brief context], I [role or responsibility].' (2) Action — 'I [specific action verb] [approach] to [collaboration outcome].' (3) REQUIRED illustration — 'So if [specific team scenario], I [specific action], which would [directional outcome].' (4) If a second dimension applies: 'I also [action].' then REQUIRED illustration. (5) Outcome — last sentence names the team or business result. No adjectives. No invented metrics. Draws from candidate background.>\n\
Close: <One sentence connecting the candidate\\'s collaboration approach to the employer\\'s specific team challenge from the system prompt. Starts with 'That\\'s why', 'This is why', or 'I\\'m confident'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Example: [1-2 word keyword] 3-5 word outcome title | <STAR story focused on collaboration or teamwork. 4 sentences max. All on ONE line. Each starts with 'I'. Situation + Action in first sentence. Team outcome in last sentence. Draws only from candidate background. No invented metrics.>\n\
Ask: <2-4 word noun phrase — team collaboration dynamic or working norm> | <Question probing how the team collaborates or handles the specific dynamic the interviewer raised. Names the concrete process, cadence, or challenge. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle on team culture or feedback> | <A different question about how the team gives feedback, resolves disagreement, or works across functions. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: one sentence naming the team or collaboration stake. Never starts with 'I'. Draws from system prompt.\n\
- Solve: one sentence starting with 'I'. Names specific working style or approach. Max 12 words.\n\
- Bridge: 5-8 words. Starts with 'I\\'d' or 'I'. Never a question.\n\
- Answer text must be on the same line as 'Answer:'.\n\
- Answer: behavioral story format. Every action claim followed immediately by inline illustration.\n\
- Close: one sentence. Max 20 words. References employer\\'s specific challenge. Never say 'this role'.\n\
- Always use 'I' — never 'we' or 'our'.\n\
- Acronyms: write in full on first use.\n\
- NEVER invent metrics, percentages, or timeframes not in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Use only background provided. No invented details.",
        ctx_prefix, question
    )
}

fn build_compound_prompt(ctx_prefix: &str, question: &str, primary: QuestionType, secondary: QuestionType) -> String {
    let topic1 = question_type_topic(primary);
    let topic2 = question_type_topic(secondary);
    format!(
        "{}The interviewer asked a compound question covering two topics: '{}'\n\n\
CRITICAL: This question asks about BOTH (1) {topic1} AND (2) {topic2}.\n\
Produce ONE cohesive spoken answer addressing BOTH dimensions in a single flowing response.\n\
Draw from ALL candidate context in the system prompt: CV, LinkedIn, portfolio, extra experience notes, early career context, job description, and company information.\n\
Use ONLY these labeled lines:\n\n\
Acknowledge: <One sentence naming BOTH dimensions — acknowledges the combined ask without restating the question. Opens with 'It sounds like you are interested in both' or 'From your question, it seems you want to understand both'. Names both {topic1} and {topic2}. Max 20 words. Never starts with 'I'. End with a period.>\n\
Answer: <Flowing answer on this same line. First 2-3 sentences address {topic1}. Each sentence starts with 'I'. Transition with 'Beyond that,' or 'What also draws me here is' or 'Building on that,'. Final 2-3 sentences address {topic2}. Max 10 words per sentence. No adjectives. No invented metrics. Draws only from candidate context in the system prompt.>\n\
Close: <One sentence connecting BOTH dimensions to the employer\\'s specific business challenge from the system prompt. Starts with \\'That\\'s why\\', \\'This is why\\', or \\'I\\'m confident\\'. Max 20 words. Never say \\'this role\\', \\'this\\', \\'it\\'.>\n\
---\n\
Ask: <2-4 word noun phrase at the intersection of {topic1} and {topic2} — names the bridging concept, not either standalone topic. e.g. \\'growth path alignment\\', \\'background fit timeline\\'> | <Question that naturally bridges BOTH dimensions — probes the specific connection between candidate background AND this role\\'s challenge. Names the concrete metric, process, or domain. Ends with \\'?\\'.> | <1 sentence. Starts with \\'I ask because\\' or \\'I\\'m curious about\\'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different bridging angle, still connecting both dimensions> | <A different question probing the intersection. Names the specific metric, process, or domain where both aspects join. Ends with \\'?\\'.> | <1 sentence. Starts with \\'I ask because\\' or \\'I\\'m curious about\\'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY: Acknowledge:, Answer:, Close:, then two Ask: lines. No other labels. No preamble.\n\
- Answer addresses {topic1} first, transitions once, then addresses {topic2}. Both parts draw ONLY from candidate background in the system prompt.\n\
- Ask topics probe the INTERSECTION of both dimensions — not topics from either standalone answer.\n\
- NEVER invent metrics, percentages, dollar figures, headcount, or timeframes not present in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.",
        ctx_prefix, question,
        topic1 = topic1,
        topic2 = topic2,
    )
}

fn build_weaknesses_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a weakness/self-awareness question: '{}'\n\n\
CRITICAL: This is a WEAKNESS question. Output ONLY the exact labeled lines below. No preamble.\n\
DO NOT deflect. DO NOT say 'I work too hard' or 'I'm a perfectionist'. Name a real, specific limitation.\n\n\
Acknowledge: <One complete sentence naming the weakness honestly. States what the limitation is AND the professional context in which it shows up — specific enough to be credible, not so severe it is disqualifying. Opens with one of: 'One area I have actively worked on is', 'A genuine limitation I have had to address is', 'If I am honest, my tendency toward'. Names the specific professional context. Max 20 words. Draws from the candidate's actual work history and background in the system prompt — CV, LinkedIn profile, extra experience notes, and early career context.>\n\
Solve: <One sentence. The specific behaviour change or structured approach the candidate adopted to address this limitation. Starts with 'I' or 'I\\'ve'. Max 15 words. Names the concrete action — not 'I am working on it'. Draws from ALL candidate context in the system prompt: CV, LinkedIn, portfolio, extra experience notes, early career context.>\n\
Bridge: <One sentence of concrete evidence from the candidate background that this approach has produced improvement. Starts with 'I'. Names the specific project, role, or context and the directional outcome — no invented metrics. Max 15 words. If no direct evidence exists in the background, use directional language: 'Since then, I have measurably improved' — never fabricate details.>\n\
Answer: <One sentence redirecting to a compensating strength that is directly relevant to the employer's specific business challenge from the system prompt (job description, company information, interviewer context). Uses [keyword] strategy format: [1-2 word keyword] + one sentence naming the strength and its business outcome for this employer. Then one concrete proof point from the candidate background (CV, LinkedIn, portfolio, extra experience). Then a directional outcome for this specific employer. No adjectives. No invented metrics.>\n\
Close: <One sentence connecting the candidate's growth trajectory to the employer's specific challenge from the system prompt. Starts with 'That growth is exactly why', 'That\\'s exactly why', or 'I\\'m confident that'. Max 20 words. References the employer\\'s actual business problem — not a generic domain. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase related to team growth or development culture> | <Question showing the candidate is actively developing — probes how the team or company supports growth in this specific area. Names a concrete process, tool, or domain. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — different angle on development or feedback> | <Question about how the team gives feedback or how performance is measured in this area. Names a specific metric or process. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text, no preamble.\n\
- Acknowledge: a real, specific limitation — never 'perfectionist' or 'work too hard'. Names the professional context where it shows up. Draws ONLY from candidate background in the system prompt.\n\
- Solve: concrete action taken, not vague aspiration. Draws from ALL candidate context: CV, LinkedIn, portfolio, extra experience notes, early career context in the system prompt.\n\
- Bridge: real evidence from background. If no direct evidence, use directional language only — never fabricate a metric, date, or outcome.\n\
- Answer: [keyword] strategy format. The compensating strength MUST be directly relevant to the employer\\'s specific business challenge from the job description and company information in the system prompt — not generic.\n\
- Close: references the employer\\'s actual problem from the system prompt. Never generic praise.\n\
- Always use 'I' — never 'we' or 'our'.\n\
- Acronyms: write in full on first use followed by abbreviation in parentheses.\n\
- NEVER invent metrics, percentages, dollar figures, headcount, or timeframes not present in the background.\n\
- NEVER name specific clients or companies — refer by industry only (e.g. 'a retail brand', 'a tech startup').",
        ctx_prefix, question
    )
}

fn build_situational_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a situational/hypothetical question: '{}'\n\n\
CRITICAL: This is a SITUATIONAL question — the candidate must demonstrate reasoning and judgment, NOT recall a past story as the main answer. Output ONLY the exact labeled lines below.\n\n\
Acknowledge: <One complete sentence spoken aloud. Names the business stakes or risk behind the scenario — not the scenario itself. Opens with: 'It sounds like the challenge here is', 'From your question, the concern seems to be', or 'I can see this scenario gets at'. Completes with the specific business outcome at stake, drawn from the job description and company context in the system prompt. Max 20 words. Never starts with 'I'. Never restates the question.>\n\
Solve: <One sentence. The candidate\\'s first-principles diagnostic lens — how they would frame or investigate the situation before acting. Starts with 'I\\'d' or 'I'. Max 12 words. Names the specific first step. Draws from the candidate\\'s actual methodology and background in the system prompt (CV, LinkedIn, portfolio, extra experience notes).>\n\
Bridge: <One short sentence transitioning from Solve to the Answer. 5-8 words. Starts with 'I\\'d' or 'I'. Never a question. Never starts with 'We' or 'Here\\'s'.>\n\
Answer: <Reasoning through the hypothetical on this same line. Each strategy MUST begin with a [1-2 word keyword] immediately before its opening sentence — no space between ] and first word. For each strategy: (A) [keyword] + one outcome sentence naming what this approach achieves. (B) 'I [action verb] [specific approach] because [why this addresses the root cause].' (C) REQUIRED illustration — 'So if [specific trigger from this scenario], I [specific action], which would [directional outcome].' Strategy 2 onward: outcome sentence opens with 'Beyond that,' or 'I also find that'. Last sentence names the overall business outcome. No upfront listing. No adjectives. No invented metrics. 2-3 strategies total. Draws from the candidate\\'s actual working methodology in the system prompt: CV roles, LinkedIn experience, portfolio evidence, and extra experience notes.>\n\
Close: <One sentence connecting the candidate\\'s reasoning approach to the employer\\'s specific business challenge from the system prompt. Draws from the job description and company information. Starts with 'That\\'s why', 'This is why', or 'I\\'m confident'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Example: [1-2 word keyword] 3-5 word outcome title | <A real past experience from the candidate background (CV, LinkedIn, extra experience notes, early career context) that demonstrates the same reasoning applied in a real situation. 3-4 sentences on ONE line. Each starts with 'I'. Max 10 words per sentence. Situation embedded in first action sentence. No invented metrics.>\n\
Ask: <2-4 word noun phrase probing the real context behind the hypothetical> | <Question that surfaces the actual business situation the interviewer had in mind. Names a specific metric, process, or recent event. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — different angle on how success is measured or what\\'s been tried> | <Question about how the team has approached this challenge previously or how success would be measured. Names a concrete metric or process. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I\\'m curious about'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Answer: candidate reasons through the hypothetical step by step — does NOT recall a story as the main answer. The Example: block may reference a real experience as supporting evidence.\n\
- Answer: draws reasoning from ALL candidate context in the system prompt: CV roles and responsibilities, LinkedIn experience, portfolio, extra experience notes, early career context, and the job description.\n\
- Each strategy in Answer MUST complete all 3 steps (keyword + outcome, action + why, illustration) before moving to the next.\n\
- Acknowledge, Solve, Close: draw from the employer\\'s actual challenge in the job description and company information.\n\
- Always use 'I' — never 'we' or 'our'.\n\
- Acronyms: write in full on first use followed by abbreviation in parentheses.\n\
- NEVER invent metrics, percentages, or numbers. Directional language only if no figure exists in the background.\n\
- NEVER name specific clients or companies — refer by industry only.",
        ctx_prefix, question
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str) -> TranscriptSegment {
        TranscriptSegment { text: text.to_string(), timestamp_ms: 0, speaker: String::new() }
    }

    #[test]
    fn no_transcript_gives_simple_prompt() {
        let p = build_user_prompt("Tell me about yourself", &[]);
        assert!(p.contains("Tell me about yourself"));
        assert!(!p.contains("Recent conversation"));
    }

    #[test]
    fn transcript_context_included() {
        let transcript = vec![seg("I worked at Acme"), seg("Then I moved to startup land")];
        let p = build_user_prompt("What's your background?", &transcript);
        assert!(p.contains("Recent conversation"));
        assert!(p.contains("Acme"));
    }

    #[test]
    fn only_last_10_segments_used() {
        let transcript: Vec<TranscriptSegment> = (0..15).map(|i| seg(&format!("seg{}", i))).collect();
        let p = build_user_prompt("question", &transcript);
        assert!(p.contains("seg14"));
        assert!(!p.contains("seg4"));
    }

    #[test]
    fn behavioral_question_uses_star_format() {
        let p = build_user_prompt("Tell me about a time you led a team", &[]);
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Solve:"));
        assert!(p.contains("Bridge:"));
        assert!(p.contains("Answer:"));
        assert!(p.contains("Example:"));
    }

    #[test]
    fn introduction_uses_career_story_labels() {
        let p = build_user_prompt("Tell me about yourself", &[]);
        assert!(p.contains("Summary:"));
        assert!(p.contains("Thread:"));
        assert!(p.contains("Story:"));
        assert!(p.contains("Next:"));
    }

    #[test]
    fn walk_me_through_background_is_introduction_not_behavioral() {
        assert!(matches!(
            classify_question("Walk me through your background").0,
            QuestionType::Introduction
        ));
    }

    #[test]
    fn motivation_uses_company_role_self_labels() {
        let p = build_user_prompt("Why are you interested in this role?", &[]);
        assert!(p.contains("Company:"));
        assert!(p.contains("Role:"));
        assert!(p.contains("Self:"));
    }

    #[test]
    fn future_uses_direction_alignment_contribution_labels() {
        let p = build_user_prompt("Where do you see yourself in 5 years?", &[]);
        assert!(p.contains("Direction:"));
        assert!(p.contains("Alignment:"));
        assert!(p.contains("Contribution:"));
    }

    #[test]
    fn closing_outputs_only_ask_lines() {
        let p = build_user_prompt("Do you have any questions for us?", &[]);
        let ask_count = p.matches("Ask:").count();
        assert!(ask_count >= 4, "Expected 4 Ask lines, got {}", ask_count);
    }

    #[test]
    fn strengths_question_uses_keyword_strategies() {
        let p = build_user_prompt("What are your strengths?", &[]);
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Answer:"));
        assert!(p.contains("[keyword]"));
        assert!(p.contains("Example:"));
    }

    #[test]
    fn competency_question_uses_full_format() {
        let p = build_user_prompt("How do you prioritize when everything is urgent?", &[]);
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Solve:"));
        assert!(p.contains("Bridge:"));
        assert!(p.contains("Answer:"));
        assert!(p.contains("Example:"));
    }

    #[test]
    fn weaknesses_question_uses_acknowledge_solve_bridge_format() {
        let p = build_user_prompt("What is your greatest weakness?", &[]);
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Solve:"));
        assert!(p.contains("Bridge:"));
        assert!(p.contains("Answer:"));
        assert!(p.contains("Close:"));
    }

    #[test]
    fn weaknesses_question_does_not_deflect() {
        let p = build_user_prompt("What is your biggest weakness?", &[]);
        assert!(p.contains("DO NOT deflect"));
        assert!(p.contains("real, specific limitation"));
    }

    #[test]
    fn weaknesses_question_references_all_candidate_context() {
        let p = build_user_prompt("Describe a weakness you have", &[]);
        assert!(p.contains("CV"));
        assert!(p.contains("LinkedIn"));
        assert!(p.contains("extra experience notes"));
        assert!(p.contains("early career context"));
    }

    #[test]
    fn situational_question_uses_reasoning_format() {
        let p = build_user_prompt("How would you handle a disagreement with your manager?", &[]);
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Solve:"));
        assert!(p.contains("Bridge:"));
        assert!(p.contains("Answer:"));
        assert!(p.contains("Example:"));
    }

    #[test]
    fn situational_question_references_all_candidate_context() {
        let p = build_user_prompt("What would you do if a project was going off track?", &[]);
        assert!(p.contains("CV"));
        assert!(p.contains("LinkedIn"));
        assert!(p.contains("portfolio"));
        assert!(p.contains("extra experience notes"));
    }

    #[test]
    fn weaknesses_classified_correctly() {
        assert!(matches!(
            classify_question("What is your greatest weakness?").0,
            QuestionType::Weaknesses
        ));
        assert!(matches!(
            classify_question("What would your manager say you need to work on?").0,
            QuestionType::Weaknesses
        ));
    }

    #[test]
    fn situational_classified_correctly() {
        assert!(matches!(
            classify_question("How would you handle a conflict with a colleague?").0,
            QuestionType::Situational
        ));
        assert!(matches!(
            classify_question("What would you do if you discovered a major bug in production?").0,
            QuestionType::Situational
        ));
    }

    #[test]
    fn situational_beats_behavioral_for_hypothetical() {
        // "how would you handle" is situational, not behavioral
        assert!(matches!(
            classify_question("How would you handle a disagreement with your team?").0,
            QuestionType::Situational
        ));
    }

    #[test]
    fn compound_question_returns_secondary_type() {
        // "Tell me about yourself and why are you interested in this role"
        // should have Introduction as primary and Motivation as secondary
        let (primary, secondary) = classify_question("Tell me about yourself and why are you interested in this role?");
        assert!(matches!(primary, QuestionType::Introduction));
        assert!(secondary.is_some());
        assert!(matches!(secondary.unwrap(), QuestionType::Motivation));
    }

    #[test]
    fn single_type_question_returns_no_secondary() {
        let (_, secondary) = classify_question("Tell me about a time you led a team");
        assert!(secondary.is_none());
    }

    #[test]
    fn compound_prompt_includes_both_topics() {
        let p = build_compound_user_prompt(
            "Tell me about yourself and why are you interested in this role?",
            &[],
            QuestionType::Introduction,
            QuestionType::Motivation,
        );
        assert!(p.contains("Acknowledge:"));
        assert!(p.contains("Answer:"));
        assert!(p.contains("Close:"));
        assert!(p.contains("career background and story"));
        assert!(p.contains("why you want this role"));
    }
}
