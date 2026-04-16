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

const FIT_TRIGGERS: &[&str] = &[
    "overqualified",
    "seem overqualified",
    "appears overqualified",
    "why junior",
    "why a junior",
    "why a lower",
    "why a more junior",
    "why are you applying for a junior",
    "why are you applying for such",
    "why apply for a junior",
    "step back",
    "step down",
    "lower level",
    "more entry-level",
    "why would you take a",
    "why would you want a junior",
    "seems like a step back",
    "seems like a step down",
    "taking a step back",
    "taking a step down",
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
    "career path",
    "career trajectory",
    "career plan",
    "fit into your career",
    "role fit into",
    "this role fit",
    "next step in your career",
    "next steps in your career",
    "your professional goals",
    "your growth goals",
    "your career direction",
    "where are you headed",
    "where do you hope to",
    "what do you hope to achieve",
    "how does this position",
    "how does this opportunity",
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

const WRAP_UP_TRIGGERS: &[&str] = &[
    "we'll be in touch",
    "we will be in touch",
    "we'll get back to you",
    "we will get back to you",
    "we'll reach out",
    "we will reach out",
    "we'll let you know",
    "we will let you know",
    "interviewing other candidates",
    "other candidates",
    "making a decision",
    "have a decision by",
    "thanks for coming in",
    "thank you for coming in",
    "thank you for your time today",
    "that's all the questions",
    "that's all my questions",
    "all the questions i have",
    "wraps up our",
    "wrapping up",
    "before we let you go",
    "before i let you go",
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

const CHARACTER_TRIGGERS: &[&str] = &[
    "what would your friends say about you",
    "what would your family say about you",
    "how would your friends describe you",
    "how would people who know you personally describe you",
    "how would someone who knows you outside of work describe you",
    "describe yourself outside of work",
    "outside of your professional life",
    "how do people who know you well",
    "what do your close friends say",
    "what would people who know you say",
    "how would you describe yourself as a person",
    "what kind of person are you",
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

// Culture = "How do you work?" — behavioural, outward-facing working style questions.
// Expects a STAR story proving the style claim. NOT for preference/requirement questions.
const CULTURE_TRIGGERS: &[&str] = &[
    "how do you collaborate",
    "how do you work with cross-functional",
    "describe your working style",
    "how do you handle conflict",
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
    "how do you motivate",
    "how do you onboard",
    "how do you give and receive feedback",
];

const VALUES_TRIGGERS: &[&str] = &[
    "what do you look for in a company",
    "what do you look for in a manager",
    "what do you look for in a team",
    "what do you look for in a role",
    "what are you looking for in",
    "what matters most to you in",
    "what is important to you in",
    "what do you need from a manager",
    "what do you need from a company",
    "what do you value in a workplace",
    "what do you value in a manager",
    "what kind of manager do you work best with",
    "what kind of leadership style",
    "what kind of environment do you thrive in",
    "what does your ideal",
    "what would your ideal company",
    "what would your ideal manager",
    "what would your ideal role",
    "what are you looking for in your next",
    "what are you looking for in a new",
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

const COMPANY_RESEARCH_TRIGGERS: &[&str] = &[
    "what do you know about us",
    "what do you know about our company",
    "what do you know about what we do",
    "have you heard of us",
    "have you heard about us",
    "have you heard of our company",
    "have you heard about our company",
    "tell me about our company",
    "tell me about what we do",
    "what can you tell me about us",
    "what can you tell me about our company",
    "how familiar are you with us",
    "how familiar are you with our company",
    "how familiar are you with what we do",
    "how familiar are you with our product",
    "have you heard about what we do",
    "what do you think of our company",
    "what do you know about the company",
    "do you know what we do",
    "what have you heard about us",
    "what do you know about our product",
    "what do you know about our work",
];

const SMALLTALK_TRIGGERS: &[&str] = &[
    "how are you",
    "how's it going",
    "how is it going",
    "how have you been",
    "how is your day",
    "how was your day",
    "how was your weekend",
    "how was your morning",
    "how are you doing",
    "how are you today",
    "nice to meet you",
    "great to meet you",
    "pleasure to meet you",
    "lovely to meet you",
    "good to meet you",
    "wonderful to meet you",
    "ready to get started",
    "shall we get started",
    "before we begin",
    "before we get started",
];

// ── Question classification ───────────────────────────────────────────────────

#[derive(Debug, Copy, Clone)]
pub enum QuestionType {
    Smalltalk,
    Introduction,
    Motivation,
    CompanyResearch,
    Fit,
    Future,
    Closing,
    WrapUp,
    Strengths,
    Weaknesses,
    Behavioral,
    Situational,
    Technical,
    Culture,
    Character,
    Values,
    Competency,
}

/// Score-based classification returning (primary, Option<secondary>).
/// Primary = highest scoring type. Secondary = runner-up with score > 0, if any.
/// Priority order (listed first = wins ties) mirrors the frontend questionTagger.ts.
pub fn classify_question(question: &str) -> (QuestionType, Option<QuestionType>) {
    let q = question.to_lowercase();

    let candidates: &[(usize, QuestionType)] = &[
        (score_triggers(SMALLTALK_TRIGGERS,         &q), QuestionType::Smalltalk),
        (score_triggers(INTRODUCTION_TRIGGERS,      &q), QuestionType::Introduction),
        (score_triggers(COMPANY_RESEARCH_TRIGGERS,  &q), QuestionType::CompanyResearch),
        (score_triggers(FIT_TRIGGERS,               &q), QuestionType::Fit),
        (score_triggers(MOTIVATION_TRIGGERS,        &q), QuestionType::Motivation),
        (score_triggers(FUTURE_TRIGGERS,            &q), QuestionType::Future),
        (score_triggers(CLOSING_TRIGGERS,           &q), QuestionType::Closing),
        (score_triggers(WRAP_UP_TRIGGERS,           &q), QuestionType::WrapUp),
        (score_triggers(STRENGTHS_TRIGGERS,         &q), QuestionType::Strengths),
        (score_triggers(WEAKNESSES_TRIGGERS,        &q), QuestionType::Weaknesses),
        (score_triggers(BEHAVIORAL_TRIGGERS,        &q), QuestionType::Behavioral),
        (score_triggers(SITUATIONAL_TRIGGERS,       &q), QuestionType::Situational),
        (score_triggers(TECHNICAL_TRIGGERS,         &q), QuestionType::Technical),
        (score_triggers(CULTURE_TRIGGERS,           &q), QuestionType::Culture),
        (score_triggers(CHARACTER_TRIGGERS,         &q), QuestionType::Character),
        (score_triggers(VALUES_TRIGGERS,            &q), QuestionType::Values),
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

    // CompanyResearch and Fit never compound — they have dedicated structures
    if matches!(primary, QuestionType::CompanyResearch | QuestionType::Fit) {
        return (primary, None);
    }

    (primary, secondary)
}

/// Kept for callers that only need a behavioral check.
pub fn is_behavioral(question: &str) -> bool {
    matches!(classify_question(question).0, QuestionType::Behavioral)
}

fn question_type_topic(qt: QuestionType) -> &'static str {
    match qt {
        QuestionType::Smalltalk        => "small talk and pleasantries",
        QuestionType::Introduction     => "your career background and story",
        QuestionType::Motivation       => "why you want this role and company",
        QuestionType::CompanyResearch  => "what you know about the company — their offering, positioning, market, growth, and backers",
        QuestionType::Fit              => "why you are applying at this level or channel — a deliberate trade-off",
        QuestionType::Future           => "your career direction and goals",
        QuestionType::Closing          => "questions you have for the interviewer",
        QuestionType::WrapUp           => "a closing statement reiterating your fit and enthusiasm",
        QuestionType::Strengths        => "your key strengths",
        QuestionType::Weaknesses       => "an area you are actively developing",
        QuestionType::Behavioral       => "a specific past behavioral example",
        QuestionType::Situational      => "how you would handle a hypothetical situation",
        QuestionType::Technical        => "your technical approach and design thinking",
        QuestionType::Culture          => "how you collaborate and work with others",
        QuestionType::Character        => "your personal qualities and how others perceive you",
        QuestionType::Values           => "what you look for in a company, manager, or role",
        QuestionType::Competency       => "your professional approach and methodology",
    }
}

pub fn question_type_to_tag(qt: QuestionType) -> &'static str {
    match qt {
        QuestionType::Smalltalk       => "smalltalk",
        QuestionType::Introduction    => "personal",
        QuestionType::Motivation      => "motivation",
        QuestionType::CompanyResearch => "company_research",
        QuestionType::Fit             => "fit",
        QuestionType::Future          => "future",
        QuestionType::Closing         => "candidate_questions",
        QuestionType::WrapUp          => "wrap_up",
        QuestionType::Strengths       => "strengths",
        QuestionType::Weaknesses      => "weaknesses",
        QuestionType::Behavioral      => "behavioral",
        QuestionType::Situational     => "situational",
        QuestionType::Technical       => "technical",
        QuestionType::Culture         => "culture",
        QuestionType::Character       => "character",
        QuestionType::Values          => "values",
        QuestionType::Competency      => "general",
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn make_ctx_prefix(transcript: &[TranscriptSegment]) -> String {
    let recent: Vec<&TranscriptSegment> = transcript.iter().rev().take(10).collect();
    let context = recent.iter().rev().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ... ");
    if context.is_empty() { String::new() } else { format!("Recent conversation: {}\n\n", context) }
}

pub fn build_user_prompt(question: &str, transcript: &[TranscriptSegment]) -> String {
    let (qtype, _) = classify_question(question);
    tracing::info!("Question type: {:?} — {:?}", qtype, question);
    let ctx_prefix = make_ctx_prefix(transcript);
    dispatch_prompt(&ctx_prefix, question, qtype, transcript)
}

pub fn build_user_prompt_for_type(question: &str, transcript: &[TranscriptSegment], qtype: QuestionType) -> String {
    let ctx_prefix = make_ctx_prefix(transcript);
    dispatch_prompt(&ctx_prefix, question, qtype, transcript)
}

/// Returns a pre-written small-talk response — no LLM call needed.
/// Variants rotate based on question length so consecutive greetings differ.
pub fn smalltalk_response(question: &str) -> String {
    const VARIANTS: &[&str] = &[
        "Tell: Doing well, thank you! Really looking forward to our conversation today.\nAsk: Morning | How has your morning been so far? |\nAsk: Week | Has it been a busy week for you? |",
        "Tell: Great, thanks for asking! I've been looking forward to this.\nAsk: Day | How is your day going so far? |\nAsk: Week | Is it a busy time of year for you? |",
        "Tell: Really well, appreciate it! Ready to go — great to meet you.\nAsk: Day | How has your day been? |\nAsk: Weather | Nice weather today — are you based locally? |",
        "Tell: Doing well, thank you for asking. Happy to be here.\nAsk: Morning | Did you have a good morning? |\nAsk: Week | Busy week for you? |",
    ];
    let idx = question.len() % VARIANTS.len();
    VARIANTS[idx].to_string()
}

pub fn build_compound_user_prompt(question: &str, transcript: &[TranscriptSegment], primary: QuestionType, secondary: QuestionType) -> String {
    const ROLE_HEADER: &str = "ROLES: YOU = the job candidate applying for this position. THEM = the employer interviewing you. You are NOT currently working there. Never say 'I work at [employer]' or use 'we'/'our' about the employer.\n\n";
    let ctx_prefix = format!("{}{}", ROLE_HEADER, make_ctx_prefix(transcript));
    build_compound_prompt(&ctx_prefix, question, primary, secondary)
}

fn dispatch_prompt(ctx_prefix: &str, question: &str, qtype: QuestionType, transcript: &[TranscriptSegment]) -> String {
    // Prepended to every prompt so small models cannot confuse candidate with employer.
    const ROLE_HEADER: &str = "ROLES: YOU = the job candidate applying for this position. THEM = the employer interviewing you. You are NOT currently working there. Never say 'I work at [employer]' or use 'we'/'our' about the employer.\n\n";
    let ctx = format!("{}{}", ROLE_HEADER, ctx_prefix);
    let ctx_prefix = ctx.as_str();
    match qtype {
        QuestionType::Smalltalk       => build_competency_prompt(ctx_prefix, question), // fallback; normally short-circuited before LLM
        QuestionType::Introduction    => build_introduction_prompt(ctx_prefix, question),
        QuestionType::Motivation      => build_motivation_prompt(ctx_prefix, question),
        QuestionType::CompanyResearch => build_company_research_prompt(ctx_prefix, question),
        QuestionType::Fit             => build_fit_prompt(ctx_prefix, question),
        QuestionType::Future          => build_future_prompt(ctx_prefix, question),
        QuestionType::Closing         => build_closing_hm_prompt(ctx_prefix, question),
        QuestionType::WrapUp          => build_wrap_up_prompt(ctx_prefix, question, transcript),
        QuestionType::Strengths       => build_strengths_prompt(ctx_prefix, question),
        QuestionType::Weaknesses      => build_weaknesses_prompt(ctx_prefix, question),
        QuestionType::Behavioral      => build_behavioral_prompt(ctx_prefix, question),
        QuestionType::Situational     => build_situational_prompt(ctx_prefix, question),
        QuestionType::Technical       => build_technical_prompt(ctx_prefix, question),
        QuestionType::Culture         => build_culture_prompt(ctx_prefix, question),
        QuestionType::Character       => build_character_prompt(ctx_prefix, question),
        QuestionType::Values          => build_values_prompt(ctx_prefix, question),
        QuestionType::Competency      => build_competency_prompt(ctx_prefix, question),
    }
}

// ── Template builders ─────────────────────────────────────────────────────────

fn build_company_research_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a company knowledge question: '{}'\n\n\
CRITICAL: This is a COMPANY RESEARCH question. Output ONLY the labeled fields below, in order.\n\
Draw from the company information in the system prompt (brief, JD, website). Where specific data is absent, synthesise plausible details from product/industry context — never leave a field empty and never say 'I don't know'.\n\
NEVER use 'we', 'our', or 'us' about the company — you are a candidate interviewing, not an employee.\n\n\
CoreOffering: <2-3 sentences. What the company builds, the core problem it solves, and the delivery model or product category. Plain language, no buzzwords. Do NOT restate the company tagline verbatim.>\n\
Positioning: <2-3 key differentiators from competitors. Each on the same line separated by ' · '. Format per item: [edge name] — one sentence on why it is defensible or hard to replicate.>\n\
Segment: <[Segment name] — one sentence on why this group has the sharpest need for the product> | <Economic Buyer title · Champion title · End User title> | <Industry1 · Industry2 · Industry3> | <headcount range or revenue/funding stage> | <Awareness headline // problem-aware copy that names their pain and links to where they can learn more // Learn more CTA text> :: <Consideration headline // copy promising a high-value gated asset they should download or register for (guide, report, or webinar) // Download or Register CTA text> :: <High-intent headline // copy for buyers ready to act now // Start free or Request demo CTA text> | <1-2 sentences: the specific friction or cost this segment experiences without a solution, from the buyer's perspective — not the product's value prop>\n\
Segment: <[Second segment name] — one sentence on why this group needs it> | <EB title · Champion title · EU title> | <Industry1 · Industry2 · Industry3> | <headcount range or stage> | <Awareness headline // awareness copy // CTA> :: <Consideration headline // consideration copy // CTA> :: <High-intent headline // high-intent copy // CTA> | <1-2 sentences: the specific friction or cost>\n\
Growth: <2-4 sentences. YoY trajectory (revenue/ARR/users growth direction), net profit trend (profitable, path to profitability, or pre-profit growth phase), one named industry headwind they navigated and how. If hard numbers are absent, use directional language only — never invent specific revenue figures.>\n\
Backers: <2-3 sentences. Most recent funding round: stage and approximate amount if known, or bootstrapped/public if applicable. 1-2 notable investor names if available. One sentence on what their investment signals: domain expertise, market confidence, strategic network, or growth thesis. If no funding data is available, say so and explain what that signals about their growth model.>\n\
Enthusiasm: <2-3 sentences. Personal hook tying the candidate's skills, domain experience, or career direction (from background in system prompt) specifically to this company's growth trajectory or mission. Must reference something concrete from CoreOffering or Growth — not generic aspiration. Each sentence starts with 'I'.>\n\
---\n\
Ask: <2-4 word noun phrase — probing a genuine aspect of the company's direction, product, or market> | <Specific grammatical question showing genuine research depth. Names a concrete challenge, growth area, or strategic decision from the system prompt. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different angle from the first> | <A different specific question about the company's trajectory, competitive position, or product direction. Names a concrete metric, challenge, or domain. Ends with '?'.> | <1 sentence. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual company context from the system prompt — never copy this example):\n\
CoreOffering: They build a revenue operations platform that connects paid marketing spend to closed pipeline for B2B sales teams. The product sits between CRM and marketing automation, aggregating attribution data so sales and marketing share a single source of truth on what is generating revenue. It is delivered as a SaaS product with API integrations to the major CRM and ad platforms.\n\
Positioning: [attribution depth] — unlike point solutions, their model tracks multi-touch paths across 12+ channels including dark social, which competitors cannot replicate at the same fidelity · [CRM-native architecture] — built to live inside Salesforce and HubSpot rather than alongside them, which eliminates the adoption friction that kills most standalone tools · [pipeline as north star] — the primary output metric is pipeline value, not MQL volume, which aligns with how CFOs now measure marketing ROI\n\
Segment: [Mid-market B2B SaaS] — they outgrow spreadsheet-based attribution before they can justify enterprise-tier tooling | VP of Revenue Ops · RevOps Manager · Account Executive | SaaS · FinTech · HR Tech | 200–1,500 employees / Series B–D | Your ops team is drowning in attribution spreadsheets // RevOps teams at your stage spend 12+ hours a week reconciling marketing data that still doesn't agree. See how teams like yours solved it. // See how it works :: The attribution playbook for $10M–$50M ARR companies // Download the guide we built with 40 RevOps leaders on what actually drives pipeline at your stage. // Download the guide :: You already know your pipeline problem — fix it this week // Book a 30-minute session and walk away with a working attribution model for your CRM. // Request a demo | Their ops team is manually stitching together attribution data across disconnected tools, burning hours on reconciliation that still produces results nobody trusts when it hits the board deck.\n\
Segment: [High-growth DTC brands] — rising customer acquisition cost makes multi-touch attribution the difference between profitable scaling and burn | CMO · Performance Marketing Director · Media Buyer | eCommerce · Consumer Subscription · Retail Tech | $5M–$50M revenue / Seed–Series B | You're scaling ad spend but can't see what's actually converting // Most DTC brands at $5M–$50M are flying blind on attribution. Here's what multi-touch measurement actually looks like at your scale. // Learn more :: Which channels drove your last 500 customers? Find out. // Join our live webinar — we'll show you how fast-growing DTC brands cut wasted spend by attributing every dollar. // Register for the webinar :: Stop guessing. Start scaling channels that actually close. // See your real attribution picture in one session — then decide where to scale next quarter. // Start free | They are scaling ad spend without reliable signal on which channels are actually closing customers, forcing gut-feel budget decisions at exactly the moment investor scrutiny on unit economics is highest.\n\
Growth: The company has grown ARR consistently over the past two years, benefiting from the shift toward pipeline-accountable marketing as CFO scrutiny on marketing budgets increased post-2022. They are in a pre-profit growth phase, investing heavily in product expansion and go-to-market reach. The industry headwind of reduced software budgets in 2023 appears to have accelerated their sales cycle as companies prioritised ROI measurement over incremental spend.\n\
Backers: Their most recent round was a Series B of approximately $20M, led by Sequoia with participation from a specialist B2B SaaS fund. Sequoia's involvement signals confidence in the scalability of the revenue operations category and access to their enterprise network for go-to-market expansion. The round also indicates the company is on a trajectory toward a Series C or eventual public market path within the next two to three years.\n\
Enthusiasm: I have spent several years working in B2B pipeline marketing and experienced the attribution problem from the inside — the gap between paid spend and pipeline outcomes was a constant constraint I built workarounds for. I'm drawn to the company's decision to make pipeline value the primary metric rather than MQL volume, which is where I believe the market is heading as finance teams take a harder look at marketing ROI. That framing maps directly to the direction I'm building toward in my career.\n\n\
Rules:\n\
- NEVER say 'I work at the company' or use 'we' / 'our' / 'us' about the company — you are a candidate.\n\
- CoreOffering: plain language — no buzzwords, no jargon, no tagline recitation.\n\
- Positioning: exactly 2-3 differentiators separated by ' · ' on a SINGLE line.\n\
- Segment: exactly 2 Segment: lines with 6 pipe-delimited sections each. Segment name in square brackets.\n\
- Growth: directional language only if hard numbers are unavailable — NEVER invent revenue or ARR figures.\n\
- Backers: if funding info is not in the system prompt, say so — NEVER invent investor names.\n\
- Enthusiasm: must reference something specific from CoreOffering or Growth — not generic.\n\
- TONE: facts and direction only — no 'passionate', 'excited', 'innovative', 'dynamic'.\n\
- ALWAYS use 'I' — never 'we' or 'our' about the candidate's employer either.\n\
- NEVER name specific companies the candidate worked at — refer by industry category only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.",
        ctx_prefix, question
    )
}

fn build_introduction_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked an introduction question: '{}'\n\n\
CRITICAL: This is an INTRODUCTION question. Use ONLY the labeled fields below.\n\
DO NOT output Solve:, Bridge:, Present:, Past:, Future:, or Answer: — those labels do not exist here.\n\n\
Acknowledge: <1 sentence. Brief natural opener acknowledging the question and signalling you will walk them through your background. Starts with 'Sure', 'Happy to', or 'Of course'. Max 10 words. Example: 'Sure — happy to walk you through my background.'>\n\
Summary: <2 sentences. Draw from ALL candidate context — CV, LinkedIn About/Summary, and Early Career & Additional Context notes — to build the broadest accurate picture of the career span. Sentence 1: broad aggregate span — 'I\\'ve spent my career in [role types] across [industries/domains].' or 'My career has spanned [role types] across [industries/domains].' Names ALL the types of roles and industries across the full career, not just the most recent. Sentence 2: the narrowing focus — 'My focus has always been on [specific area].' or 'But I\\'ve mainly focused on [specific area].' or 'With a particular focus on [specific skill or domain].' Max 10 words per sentence. Speak as naturally said aloud — not a resume bullet.>\n\
Thread: <1 sentence. The connecting quality or role the candidate plays that runs through all those experiences AND makes them the natural fit for this employer's specific challenge (from the system prompt). Do NOT just name a skill — name the pattern of impact or the role they play for others (e.g. 'being the person teams rely on to turn ambiguous briefs into measurable results'). Starts with 'The throughline of my career has been', 'Throughout all of that,', 'What connects all of it is', or 'At the heart of all of it is'. Must land as relevant to what this employer needs — not just a personal brand statement. Max 20 words.>\n\
Transition1: <1 sentence bridging Thread to Story. Starts with 'Let me walk you through how I got here.' or 'That started with' or 'Earlier in my career,'. Max 10 words.>\n\
Story: <2-3 sentences. Each starts with 'I'. Insight-driven past moves — the formative experiences that shaped the candidate. Draw from ALL candidate context: CV job entries, LinkedIn About/Summary section, and Early Career & Additional Context notes. These may contain early roles that predate the CV. Use framing like: 'That role showed me I was passionate about [X].' or 'After that I learned it was important to be at a company that values [X].' or 'That was a foundational experience when it comes to [X].' No invented details.>\n\
Transition2: <1 sentence bridging Story to Next. Starts with 'Which is why,' or 'That path led me to' or 'Building on that,'. Max 10 words.>\n\
Next: <1-2 sentences. Each starts with 'I'. Forward momentum: what you are currently focused on or looking to do next. Use framing like: 'As the next step I\\'m looking to...' or 'What I\\'m currently focused on is...' or 'Looking ahead I\\'m focused on...'. Names the specific direction — not vague aspiration.>\n\
Transition3: <1 sentence bridging Next to Close. Starts with 'So when I came across' or 'That context is exactly why' or 'Which is what led me here'. Max 10 words.>\n\
Close: <2 sentences. First: connect your Next to why this specific employer and challenge appeals, referencing the employer\\'s problem from the system prompt. Starts with 'Given that I\\'m focused on' or 'Since my next step is about' or 'As I look to [next step],'. Second: enthusiasm bridge — exactly 'And that\\'s why I\\'m so excited to be speaking with you today.' Max 35 words total. Never say \\'this role\\', \\'this\\', \\'it\\' as vague pronouns.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you\\'re asking about — e.g. 'team structure', 'client mix', 'success metrics'> | <Specific grammatical question the candidate asks the interviewer. Directly related to the type of work or challenge described by the interviewer. Ends with '?'.> | <1 sentence if asked 'why do you ask?' — starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic. Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase> | <A different specific question about the opportunity. Names the concrete topic — no vague pronouns. Ends with '?'.> | <1 sentence follow-up. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: Sure — happy to walk you through my background.\n\
Summary: I've spent my career in performance marketing across B2B technology companies. My focus has always been on paid acquisition tied directly to pipeline and revenue.\n\
Thread: The throughline of my career has been connecting paid channel budgets to measurable pipeline outcomes for sales-led businesses.\n\
Transition1: That started early in my career.\n\
Story: I began in paid search at a software company, where I learned to link ad spend directly to closed revenue. I moved into multi-channel roles after seeing attribution gaps cost teams budget they could not explain.\n\
Transition2: Which is why\n\
Next: I am now focused on cross-channel paid strategy that is fully accountable to pipeline, not just volume.\n\
Transition3: So when I came across this opportunity,\n\
Close: Given that I'm focused on pipeline-accountable acquisition, the growth challenge in your market is exactly where I want to build next. And that's why I'm so excited to be speaking with you today.\n\
Ask: pipeline attribution model | How does the team currently connect paid channel spend to pipeline and closed revenue? | I ask because pipeline attribution drives every channel prioritisation decision I make.\n\
Ask: paid channel scope | Does this role own the full paid channel mix, or are specific channels managed separately? | I'm curious about the scope because cross-channel efficiency is where I've found the highest leverage.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Summary: Thread: Transition1: Story: Transition2: Next: Transition3: Close: Ask: Ask:\n\
- TONE: facts and direction only — no adjectives, no 'passionate', 'excited', 'dedicated', 'driven'.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- NEVER invent metrics, percentages, figures, or timeframes not in the background.\n\
- NEVER name specific companies or clients — refer by industry only.\n\
- ACRONYMS: write in full on first use, abbreviation in parentheses.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
        ctx_prefix, question
    )
}

fn build_motivation_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a motivation question: '{}'\n\n\
CRITICAL: This is a MOTIVATION question. Output ONLY the labeled lines below.\n\
DO NOT output Solve:, Bridge:, or Answer: — those labels do not exist here.\n\n\
Acknowledge: <1 sentence. Brief natural opener acknowledging the question and signalling genuine thought about this opportunity. Starts with 'Sure', 'Happy to', or 'Of course'. Max 10 words. Example: 'Happy to — I have been thinking about this carefully.'>\n\
Company: <1-2 sentences. Describes THEIR challenge — what problem the employer is trying to solve, drawn ONLY from the system prompt. NEVER say 'I work at [employer]' — you are applying there, not employed there. NEVER invent tools, platforms, or strategies not in the system prompt. Each sentence starts with 'I see that', 'I notice that', or 'I understand that'. Max 10 words each.>\n\
Transition1: <1 sentence connecting Company to Role. Starts with 'That challenge maps directly to' or 'Which is where' or 'My background fits because'. Max 10 words.>\n\
Role: <2 sentences. Each starts with 'I'. Max 10 words each. How your specific background maps to this role's requirements. Draw only from background provided. No invented details.>\n\
Transition2: <1 sentence connecting Role to Self. Starts with 'But beyond the skillset,' or 'On a personal level,' or 'And what draws me further is'. Max 10 words.>\n\
Self: <1-2 sentences. Each starts with 'I'. Max 10 words each. How this role fits your career trajectory. What you will build or develop here that you cannot elsewhere.>\n\
Transition3: <1 sentence connecting Self to Close. Starts with 'So when I look at' or 'That combination is exactly why' or 'Which is why'. Max 10 words.>\n\
Close: <One sentence. Connects your motivation to the employer's specific challenge from the system prompt. Starts with 'That\'s why', 'This is why', 'The work I\'ve done in', or 'What I\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn from the specific topic the interviewer raised. e.g. 'business challenge', 'client mix', 'team priorities'> | <Specific grammatical question probing an aspect of what the interviewer asked about. Names a concrete challenge, outcome, or constraint from the system prompt. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different angle from the first, still related to the interviewer's question> | <A different specific question. Names a concrete metric, process, or domain — no vague pronouns. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: Happy to — I've been thinking about this carefully.\n\
Company: I see that the company is focused on scaling paid acquisition in a competitive market. I understand the core challenge is generating qualified pipeline at a sustainable cost.\n\
Transition1: My background fits because\n\
Role: I've spent several years running paid acquisition for B2B technology companies with exactly that constraint. I've managed campaigns where cost per pipeline opportunity was the primary success metric.\n\
Transition2: But beyond the skillset,\n\
Self: I am looking to own the full acquisition funnel rather than individual channels. This role is the next step from channel execution to acquisition strategy.\n\
Transition3: Which is why\n\
Close: What I'd bring here specifically is the ability to connect paid spend to pipeline outcomes — which is the core challenge the job description is asking someone to solve.\n\
Ask: funnel conversion rate | What does the current conversion rate look like from paid lead to qualified sales opportunity? | I ask because that ratio tells me where campaign optimisation has the highest leverage.\n\
Ask: channel strategy ownership | Does this role set channel mix strategy, or is the brief to optimise within a defined channel set? | I'm curious because channel strategy ownership is the direction I'm building toward.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Company: Transition1: Role: Transition2: Self: Transition3: Close: Ask: Ask:\n\
- Company: describes THEIR challenge from the system prompt — never 'great company'. NEVER say 'I work at [employer]' — you are applying there. NEVER invent tools or strategies.\n\
- Role: draws ONLY from candidate background — NEVER invent skills, tools, or experience not in the background.\n\
- TONE: facts and direction only — no adjectives, no 'passionate', 'excited', 'dedicated', 'driven'.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- NEVER invent metrics, figures, or timeframes not in the background. Directional language only.\n\
- NEVER name specific companies or clients — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
        ctx_prefix, question
    )
}

fn build_fit_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a level/fit challenge question: '{}'\n\n\
CRITICAL: This is a FIT/TRADE-OFF question — the interviewer is questioning why the candidate is applying at this level or in this channel. \
Output ONLY the labeled lines below. Never be defensive.\n\
DO NOT output Overqualified:, Company:, Motivation:, Role:, Trade:, Value:, or Answer: — those labels do not exist here. Start directly with Acknowledge:.\n\n\
Acknowledge: <1 sentence. Briefly confirm ONLY the level or seniority observation the interviewer made. Do NOT name any skills, channels, tools, or experience gaps here — those belong in Gap. Just confirm the premise calmly. Starts with 'That\\'s right', 'You\\'re right', or 'Fair observation'. Max 12 words. Example for 'why a more entry-level position': 'You\\'re right — my background is at a more senior level.'>\n\
Reframe: <1-2 sentences. Flip the premise — name the level or channel difference as a deliberate choice, not a retreat. Starts with 'I'. Tone: matter-of-fact. Max 10 words each.>\n\
Transition1: <1 sentence bridging Reframe to Gap. Starts with 'Specifically,' or 'The area I am building toward is' or 'What the JD asks for that I am actively developing is'. Max 12 words.>\n\
Gap: <1-2 sentences. From the employer's perspective: name the SPECIFIC skill, channel, or domain listed in the JD where the candidate's background is absent or materially thinner. Read the JD requirements in the system prompt carefully — if the JD asks for channel X and the candidate lacks X, name X explicitly and tactfully. Starts with 'I'. Max 10 words each.>\n\
Transition2: <1 sentence bridging Gap to Choice. Starts with 'I chose this employer specifically because' or 'This organisation is the right place to build it because' or 'The reason this role in particular is'. Max 12 words.>\n\
Choice: <1-2 sentences. Why this specific employer and role is the right place to address this gap — connects the JD challenge to the candidate's strategic intent. Draws from the employer's specific challenge in the system prompt. Starts with 'I'. Max 10 words each.>\n\
Transition3: <1 sentence bridging Choice to Bring. Starts with 'What I bring in the meantime is' or 'And what I contribute from existing depth is' or 'Where I add immediate value is'. Max 12 words.>\n\
Bring: <1-2 sentences. What the candidate contributes from existing depth that a career junior can't: faster ramp, cross-channel perspective, stakeholder credibility, pattern recognition from adjacent domains. Names the specific skill or perspective — not vague. Starts with 'I'. Max 10 words each.>\n\
Close: <One sentence. Connects the whole framing to the employer's specific challenge from the system prompt. Starts with 'That\\'s why', 'This is why', 'The work I\\'ve done in', or 'What I\\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
----\n\
Ask: <2-4 word noun phrase> | <Question> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\
Ask: <2-4 word noun phrase> | <Question> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background and JD — structure is fixed):\n\
Acknowledge: You're right — my background is at a more senior level.\n\
Reframe: I am choosing this level deliberately. I am building depth in a channel where my background is currently thinner.\n\
Transition1: Specifically, the area I am building toward is\n\
Gap: I have limited hands-on experience in consumer-facing paid social acquisition. My background is B2B performance marketing, so consumer acquisition is the gap I need to close.\n\
Transition2: I chose this employer specifically because\n\
Choice: The company's focus on performance advertising gives me direct exposure to consumer acquisition at scale. This environment is the right place to develop that depth alongside existing strengths.\n\
Transition3: What I bring in the meantime is\n\
Bring: I contribute cross-channel measurement and attribution knowledge that a career junior typically does not have. I also bring pipeline-linked thinking that sharpens how we evaluate consumer funnel quality.\n\
Close: That's why this role sits at the exact intersection where my existing depth and the gap I need to close are both in play.\n\
Ask: channel onboarding structure | How does the team onboard someone with adjacent but not direct channel experience? | I ask because I want to understand how quickly I can contribute independently while building the new channel depth.\n\
Ask: performance feedback cadence | How frequently does the team review channel decisions and give feedback on optimisation calls? | I'm curious because tighter feedback loops accelerate skill development in a channel that's new to me.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Reframe: Transition1: Gap: Transition2: Choice: Transition3: Bring: Close: Ask: Ask:\n\
- Acknowledge: confirms level/seniority observation ONLY — no skills, channels, or JD specifics (those go in Gap). Never repeats Reframe.\n\
- CRITICAL — Gap: name the SPECIFIC skill or channel from the JD where background is absent (e.g. 'paid social', 'SEO') — never 'a new area'.\n\
- Choice: draws from the employer's actual challenge in the system prompt — not generic.\n\
- Bring: draws ONLY from candidate background — NEVER invent skills, tools, or experience not in the background.\n\
- TONE: facts and trade-offs only — no adjectives, no 'passionate', 'excited', 'dedicated'.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- NEVER invent metrics, percentages, or timeframes.\n\
- NEVER name specific companies or clients — refer by category only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
        ctx_prefix, question
    )
}

fn build_future_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a future/growth question: '{}'\n\n\
CRITICAL: This is a FUTURE/GROWTH question. Output ONLY the labeled lines below.\n\
DO NOT output Solve:, Bridge:, or Answer: — those labels do not exist here.\n\n\
Acknowledge: <1 sentence. Brief natural opener acknowledging the question and signalling you have given this genuine thought. Starts with 'Sure', 'Happy to', or 'Of course'. Max 10 words. Example: 'Sure — I have given this real thought.'>\n\
Direction: <2-3 sentences. Each starts with 'I'. Max 10 words each. (1) Qualitative shift in ownership, scope, or influence — not 'doing more of what I already do' (execution → commercial strategy, specialist → cross-channel architect, individual contributor → team or portfolio lead). Read the CV to understand current level; write the next genuine level up. (2) How that path connects to this specific employer and role. Starts with 'That path leads directly to' or 'Which is why this employer'. Never 'dream job' or 'passionate'. Never name specific tools or platforms — say 'paid channels', 'performance media' instead.>\n\
Alignment: <2 sentences. Each starts with 'I'. Max 10 words each. How this role and employer sit on that path. (1) Names the employer's specific challenge or growth area from the system prompt. (2) What you will concretely build or deliver here. Starts with 'Concretely,' or 'In practical terms,' for sentence 2.>\n\
Close: <One sentence. Connects your trajectory to the employer's specific challenge from the system prompt. Starts with 'That\'s why', 'This is why', 'The work I\'ve done in', or 'What I\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn from the specific topic the interviewer raised. e.g. 'growth path', 'skill development', 'team challenge'> | <Specific grammatical question probing an aspect of what the interviewer asked about. Names a concrete skill, domain, or outcome. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different angle, still related to the interviewer's question> | <A different specific question about the opportunity or challenge ahead. Names a concrete metric, process, or domain. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: Sure — I have given this real thought.\n\
Direction: I see myself moving from channel execution into commercial acquisition strategy — owning the connection between paid spend and revenue outcomes rather than managing individual campaigns. That path leads directly to roles like this one, where the brief is building acquisition capability that is accountable to pipeline.\n\
Alignment: I see the company is at an inflection point where paid acquisition needs to scale without proportional cost increases. Concretely, I would build the measurement infrastructure that makes that possible — attribution frameworks and funnel tracking tied to actual revenue.\n\
Close: The work I've done in pipeline-linked paid acquisition is exactly what's needed to make that growth efficient and defensible here.\n\
Ask: growth investment horizon | How long does the company expect before paid acquisition is generating a self-sustaining pipeline contribution? | I ask because that timeline shapes how I would sequence the measurement buildout and channel prioritisation.\n\
Ask: team structure at scale | Is the intent to build a larger paid media team as acquisition scales, or stay lean with more tooling? | I'm curious because the team model determines how I would structure campaign architecture from the start.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Direction: Alignment: Close: Ask: Ask:\n\
- CRITICAL — Direction must describe the NEXT level ABOVE current documented experience — not repeat what the CV already shows.\n\
- Alignment: names the employer's actual challenge from the system prompt — not generic.\n\
- TONE: facts and direction only — no adjectives, no 'passionate', 'excited', 'dedicated', 'driven', no 'dream'.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- NEVER invent metrics, figures, or timeframes not in the background.\n\
- NEVER name specific companies, clients, tools, or platforms — refer by category only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
        ctx_prefix, question
    )
}

fn make_wrap_up_context(transcript: &[TranscriptSegment]) -> String {
    // Skip the last 4 segments (the farewell exchange itself), then take up to 30
    // substantive segments — filtering out very short filler responses (< 6 words).
    let skip_tail = 4usize;
    let end = transcript.len().saturating_sub(skip_tail);
    let substantive: Vec<&TranscriptSegment> = transcript[..end]
        .iter()
        .filter(|s| s.text.split_whitespace().count() >= 6)
        .collect();
    let take = substantive.len().min(30);
    let relevant = &substantive[substantive.len().saturating_sub(take)..];
    if relevant.is_empty() { return String::new(); }
    let formatted = relevant.iter()
        .map(|s| format!("{}: {}", s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n");
    format!("Interview conversation (most recent exchanges):\n{}\n\n", formatted)
}

pub fn build_wrap_up_prompt(ctx_prefix: &str, question: &str, transcript: &[TranscriptSegment]) -> String {
    let interview_ctx = make_wrap_up_context(transcript);
    format!(
        "{}{}The interviewer has signalled the interview is ending: '{}'

Output ONLY the four labeled lines below, in order. No preamble, no commentary, no extra lines.

---
Thanks: <One complete spoken sentence. Warm, genuine, not effusive. Must start with 'Thank you' or 'It's been' or 'I really enjoyed'. Max 12 words.>
Reiterate: <One complete spoken sentence. Must start with 'The work I've done in', 'I've been doing', or 'What I'd bring specifically is'. Name the candidate's single most relevant concrete qualification and connect it to the employer's specific challenge. No vague claims like 'my experience' or 'my background'. Max 20 words.>
Echo: <One complete spoken sentence. Must start with 'I especially enjoyed' or 'The conversation about' or 'I appreciated'. Reference a specific topic, question, or moment from the interview. If no context is available, name a concrete aspect of the role. Max 20 words.>
Forward: <One complete spoken sentence. Must start with 'I look forward to' or 'I'm excited to hear' or 'I'd love to'. Express genuine anticipation for next steps or contributing. Never uses 'this opportunity'. Max 15 words.>

Rules:
- Every line must be a grammatically complete sentence a person would naturally say aloud.
- Reiterate must name a real, specific qualification — not 'my experience' or 'my background'.
- Echo must reference something specific from the conversation or role.
- Total spoken time: under 30 seconds.",
        ctx_prefix, interview_ctx, question
    )
}

pub fn build_closing_hr_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a closing question: '{}'

\nCRITICAL: Output ONLY the --- separator followed by exactly 4 Ask: lines. No other content.

\n---
\nAsk: <2-4 word noun phrase — onboarding and ramp-up> | <Question about how the first 90 days are structured or what support is in place. Names something specific. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — what high performers share> | <Question about what the most successful people in this role or team have in common — beyond skills. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — career growth path> | <Question about what progression from this role looks like — what opens up, rough timelines, what signals readiness. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — day-to-day culture> | <Question about how the company's or team's values show up concretely in day-to-day work — not just on paper. Names something specific. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>

\nRules:
\n- Ask topics: 2-4 word noun phrases. Never verb phrases. Never vague.
\n- Every question names a specific metric, process, or outcome — never 'this', 'it', or vague pronouns.
\n- NEVER name specific clients or companies. Refer by industry only.",
        ctx_prefix, question
    )
}

pub fn build_closing_hm_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a closing question: '{}'

\nCRITICAL: Output ONLY the --- separator followed by exactly 4 Ask: lines. No other content.

\n---
\nAsk: <2-4 word noun phrase — the employer's core challenge, drawn from the system prompt> | <Question probing the employer's specific business or technical challenge. Names the exact problem from the system prompt. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — 90-day success definition> | <Question about what outcomes or deliverables define a strong start in this exact role. Names the specific outcome, metric, or deliverable. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — team decision-making process> | <Question about how the team makes decisions in the relevant domain. Names the specific process, tool, or domain. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — company or team direction> | <Forward-looking question about where the company or team is headed. Names the specific domain or growth area. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>

\nRules:
\n- Ask topics: 2-4 word noun phrases. Never verb phrases. Never vague.
\n- Every question names a specific metric, process, tool, domain, or outcome — never 'this', 'it', or vague pronouns.
\n- Draw on the employer's challenge and role details from the system prompt — not generic interview questions.
\n- NEVER name specific clients or companies. Refer by industry only.",
        ctx_prefix, question
    )
}

pub fn build_closing_ceo_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a closing question: '{}'

\nCRITICAL: Output ONLY the --- separator followed by exactly 4 Ask: lines. No other content.

\n---
\nAsk: <2-4 word noun phrase — company vision or strategic direction> | <Question about where the CEO sees the company in 3-5 years, or the most important strategic bet they are making. Names something specific from the system prompt context. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — biggest current challenge> | <Question about what challenge at the company level keeps them most focused right now. Names the market, product, or operational domain from the system prompt. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — what this hire unlocks> | <Question about what this role makes possible for the company that wasn't possible before — what gap it fills at the company level. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>
\nAsk: <2-4 word noun phrase — what success looks like to them> | <Question about what kind of person moves the needle at this company from the CEO's perspective — what traits or behaviours they most value. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>

\nRules:
\n- Ask topics: 2-4 word noun phrases. Never verb phrases. Never vague.
\n- Every question names a specific domain, challenge, or outcome — never 'this', 'it', or vague pronouns.
\n- Draw on the company's challenge and context from the system prompt.
\n- NEVER name specific clients or companies. Refer by industry only.",
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
Close: <One sentence connecting your strengths to the employer's specific business challenge. Starts with 'That\'s why', 'This is why', 'The work I\'ve done in', or 'What I\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this work', 'this', 'it', or 'that'.>\n\
---\n\
Ask: <2-4 word noun phrase — directly related to what the interviewer asked about strengths. e.g. 'highest impact area', 'skill application', 'team gap'> | <Question probing where the specific strengths just discussed would have the most impact — names the domain, metric, or challenge from the system prompt. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different angle on what the interviewer raised> | <A different question about what the team most needs — names the specific skill area or outcome. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like this role needs someone who can connect paid channel performance to pipeline outcomes — which is the core of what I've built my career on.\n\
Answer: [data analysis]Identifying which metric actually drives the business outcome separates campaigns that scale from campaigns that just spend. At a B2B software company, I rebuilt attribution tracking that revealed a significant budget misallocation toward high-spend, low-pipeline channels. For the growth challenge here, this means spend decisions would be tied to pipeline contribution, not just volume.[campaign architecture]Beyond that, building a structured account framework prevents data loss as campaigns scale. I implemented a naming and tagging standard across paid channels that allowed clean performance comparison and faster optimisation decisions. For a team scaling acquisition, this means every insight starts from clean, comparable data. The combined business value of these strengths is a paid acquisition function that earns budget through measurable pipeline impact.\n\
Close: What I'd bring here specifically is the ability to build an acquisition engine that leadership can defend with revenue data, not just media metrics.\n\
Ask: highest-impact skill area | Where would strong attribution analysis have the most direct impact on campaign decisions here? | I ask because understanding where measurement is weakest tells me where I can contribute from day one.\n\
Ask: team skill gap | What analytical capability is the team currently missing most? | I'm curious because knowing the gap helps me understand where my depth creates the most immediate value.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Answer: Close: Ask: Ask:\n\
- Acknowledge: names business priority behind the question — never starts with 'I'.\n\
- Answer: [keyword] immediately before each strength sentence — no space between ] and first word. 2-3 strengths only.\n\
- Answer: evidence draws ONLY from candidate background — NEVER invent metrics, tools, or skills. Directional language only.\n\
- Answer: connection names the employer's actual challenge from the system prompt.\n\
- TONE: facts and outcomes only — no adjectives, no 'passionate', 'utilize'.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- ACRONYMS: write in full on first use, abbreviation in parentheses.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
Answer: [Situation]<One sentence of brief context. 'In [context], I [role or task].' Max 12 words.> [Task]<One sentence naming what needed to be achieved or resolved. Starts with 'The goal was' or 'I needed to'. Max 12 words.> [Action]<Two to three sentences. First: specific action taken — starts with 'I [verb]'. REQUIRED: immediately follow with inline illustration — 'So if [specific trigger], I [specific action], which would [directional outcome].' If a second action applies, use 'I also [action].' then another illustration. No adjectives. No adverbs. No 'utilize'. Name metrics, channels, tools, processes explicitly.> [Result]<One sentence naming the directional outcome achieved. Starts with 'As a result,' or 'The outcome was' or 'This led to'. Directional language only — never invent a metric.>\n\
Close: <One sentence the candidate says after the Answer. Mirrors the employer\'s specific business challenge from the system prompt — name the exact problem the employer is trying to solve (their growth constraint, market challenge, or operational goal) not a generic domain. Starts with 'That\'s why', 'This is why', 'The work I\'ve done in', or 'What I\'d bring here specifically is'. Max 20 words. End with a period. Never say 'this role', 'this work', 'this', 'it', or 'that'.>\n\
---\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn directly from the topic the interviewer raised. e.g. 'team prioritization', 'success metrics', 'client feedback loop'> | <A genuine question the candidate asks the interviewer. Names a specific metric, tool, process, or concept related to what the interviewer asked about. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different angle, still related to the interviewer's question> | <A different genuine question. Names the specific topic — no vague pronouns. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like the team is focused on maintaining campaign performance during periods of changing strategy or stakeholder priorities.\n\
Solve: I've managed paid campaigns through mid-flight brief changes without losing pipeline contribution.\n\
Bridge: I'd walk through how I approach that.\n\
Answer: [Situation]In a B2B software campaign, I managed paid acquisition when the target segment shifted mid-flight. [Task]I needed to redirect spend without losing pipeline momentum or wasting committed budget. [Action]I paused automated bidding to prevent the algorithm from optimising toward the wrong audience, then rebuilt targeting criteria based on the revised segment definition. So if the algorithm had continued on stale signals, conversion rates would have dropped and the budget shift would have been invisible until the next reporting cycle. I also updated pipeline tracking tags to reflect the new segment, which allowed clean measurement from the restart date forward. [Result]As a result, pipeline contribution was maintained within the quarter and the new segment data was clean from day one.\n\
Close: The work I've done managing mid-campaign pivots in performance accounts is directly applicable to keeping acquisition on track when strategy shifts here.\n\
Ask: brief change frequency | How often do campaign targeting or audience briefs change after launch, and what typically triggers the change? | I ask because the frequency determines how much adaptability buffer I need to build into campaign architecture.\n\
Ask: stakeholder alignment process | When a brief changes, who confirms the revised audience or success criteria before campaigns restart? | I'm curious about the decision chain because knowing it upfront prevents the alignment delays that cost the most time.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Solve: Bridge: Answer: Close: Ask: Ask:\n\
- Acknowledge: names business priority — never starts with 'I'. Never restates question.\n\
- Solve: starts with 'I'. Names specific business outcome from Acknowledge. Max 12 words.\n\
- Bridge: 5-8 words. Starts with 'I\'d' or 'I'. Never a question.\n\
- Answer: use exactly [Situation] [Task] [Action] [Result] keywords in order. All on same line as 'Answer:'.\n\
- Answer [Action]: EVERY action claim MUST be immediately followed by inline illustration — 'So if [trigger], I [action], which would [outcome].'\n\
- Answer [Result]: directional outcome only — NEVER invent metrics, tools, or outcomes not in the background.\n\
- TONE: no adjectives or adverbs. No 'utilize'. Facts and actions only.\n\
- ALWAYS use 'I' — never 'we', 'our team'. Candidate speaks only about their own actions.\n\
- ACRONYMS: write in full on first use, abbreviation in parentheses.\n\
- NEVER invent metrics, tools, skills, or details not explicitly in the candidate background.\n\
- NEVER name specific clients, employers, or companies — refer by industry only.\n\
- If employer is agency/consultancy: frame answers as client work across accounts — never owning one company's strategy.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
        ctx_prefix, question
    )
}

fn build_values_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a values/preferences question: '{}'\n\n\
CRITICAL: This question asks what environment helps the candidate do their best work. \
Draw ONLY from themes explicitly stated in the candidate's uploaded experience notes, CV, and LinkedIn in the system prompt. \
Do NOT invent preferences that are not in the background. Do NOT substitute generic preferences like 'ownership' or 'direction' unless the background specifically mentions them. \
Output ONLY the exact labeled lines below. No preamble.\n\n\
STEP 1 — IDENTIFY: Read the '## Early Career & Additional Context' section in the system prompt. That section contains the candidate's explicitly stated preferences. \
List the 3 preference themes that appear most clearly there — use the candidate's actual words (e.g. if they wrote 'growth', 'innovation', 'collaborative environment', use those exact themes, not substitutes like 'ownership' or 'direction').\n\n\
STEP 2 — REFRAME: Translate each identified preference into a contribution condition — how that environment helps the candidate deliver their best work. \
NEVER quote raw phrases like 'consistent leadership' or 'stable leadership' — reframe as 'clear direction from leadership'. \
'no micromanagement' → 'autonomy to execute'; 'flat hierarchy' → 'direct access to decision-makers'; 'work-life balance' → 'sustainable pace'. \
Never imply a gap or deficiency in the company. Always leave room for the company to meet the need in their own way.\n\n\
Acknowledge: <One sentence naming the underlying fit dimension the interviewer is probing — e.g. leadership style, culture, growth environment. \
Opens with 'It sounds like', 'From your question, the priority seems to be', or 'I can see the focus here is on'. Max 20 words. Never starts with 'I'.>\n\
Solve: <All on this same line. Short sentences. Max 10 words per sentence. Output 3 concrete preferences drawn from the candidate's uploaded background. \
Reframe each preference as what helps the candidate contribute, not what the company must provide. e.g. instead of 'I need consistent leadership', say 'I do my best work when direction is clear'. \
Each preference MUST begin with a [1-2 word keyword] immediately before its first sentence — no space between ] and the first word. The keyword is a short noun phrase, never a transition phrase like 'Beyond that'. \
Each preference follows this exact 3-part pattern: (A) [keyword] + one sentence on what conditions help the candidate contribute most. \
(B) 'I [verb] [approach] because [why — reference the most specific documented experience from the background. Max 20 words. Prefer a specific project or situation over a general pattern. \
COMPANY NAMING RULE: only name a company when the reference is positive (e.g. growth, achievement, skill built). If the experience is negative or explains a gap (e.g. inconsistent management, lack of support), write 'in a previous role' or 'at a previous employer' — never name the company.]' \
(C) 'So if [this company/team has X or does Y], I [would/can do Z], which would [positive outcome for both].' — CRITICAL: the outcome in (C) must make sense for the CURRENT ROLE being interviewed for (use the job description in the system prompt). Do NOT carry over outcomes from previous roles — e.g. do not mention 'campaign rejections' if the current role is performance marketing, not content review. The outcome should reflect what success looks like in this specific role.\
Preferences 2 and 3: the (A) sentence opens with 'Beyond that,' or 'I also find that' — placed AFTER the [keyword] marker, not before it. e.g. '[ownership]Beyond that, clear ownership reduces decision lag.' No adjectives. No adverbs. No invented details.>\n\
Bridge: <One sentence connecting the candidate's preferences to this specific employer. 5-8 words. Starts with 'I\\'d' or 'I'. Never a question.>\n\
Close: <One sentence. States why this employer or team specifically fits what the candidate looks for, drawn from the company context in the system prompt. \
Starts with 'That\\'s why', 'This is why', 'The work I\\'ve done in', or 'What I\\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase — specific aspect of the thing the interviewer asked about (company/manager/team)> | \
<Question asking the interviewer to describe that specific dimension of the company or team. Ends with '?'.> | \
<1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different aspect> | \
<A different question probing another dimension relevant to the candidate\\'s stated preferences. Ends with '?'.> | \
<1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like the focus here is on finding someone whose working style fits how this team operates.\n\
Solve: [clarity]I do my best work when direction is clear. I moved into a project lead role at a previous employer because clear scope reduced rework. So if this team sets explicit sprint goals, I can own delivery without daily check-ins, which would keep the team moving faster.[autonomy]Beyond that, I find that ownership over my work area drives quality. I restructured a content pipeline at a previous employer because shared ownership meant errors went uncaught. So if this team assigns clear domain ownership, I can catch issues earlier, which would reduce revision cycles.[feedback]I also find that regular feedback loops accelerate growth. I initiated bi-weekly peer reviews at a previous employer because async feedback alone missed nuance. So if this team runs structured peer reviews, I can iterate faster, which would shorten time to impact on new work.\n\
Bridge: I'd bring that same approach here.\n\
Close: That's why this team's emphasis on clear ownership and direct feedback aligns with how I work best.\n\
Ask: sprint goal clarity | How does the team set and communicate sprint priorities when timelines shift? | I ask because clear goal alignment is what lets me take ownership without needing check-ins.\n\
Ask: feedback cadence | How does the team typically give and receive peer feedback? | I'm curious about whether the team's review rhythm leaves room to iterate quickly.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Solve: Bridge: Close: Ask: Ask:\n\
- Solve: draw ONLY from themes in candidate's uploaded background — use their actual stated themes, NOT generic substitutes.\n\
- Solve: reframe each preference as a contribution condition — e.g. 'consistent leadership' → 'clear direction from leadership'.\n\
- Solve: each preference = [keyword] + 3-part pattern: (A) condition; (B) 'I [verb] because [specific background experience]'; (C) 'So if [company does X], I [would do Y], which would [outcome for THIS role]'.\n\
- Solve (C): outcome MUST reflect the current role from the job description — NOT outcomes from past roles.\n\
- Solve: preferences 2 and 3: (A) sentence opens with 'Beyond that,' or 'I also find that' — placed AFTER [keyword], never before.\n\
- COMPANY NAMING: only name past employers for positive references. For negative/gap experiences, use 'a previous role'.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- NEVER invent preferences, values, or experiences not in the candidate background.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
Answer: <The spoken answer on this same line. Short sentences. Max 10 words per sentence. Each strategy MUST begin with a [1-2 word keyword] immediately before its outcome sentence — no space between ] and the first word of the outcome. e.g. '[targeting] Customer acquisition cost rises when targeting drifts. I audit targeting and form messaging because drift is the most common cost driver. So if a client\'s CPA rises, I audit targeting and form changes, which would bring conversion costs down.[attribution] Attribution gaps hide which channels drive revenue. I implement multi-touch attribution because single-touch models misallocate budget. So if ROAS drops, I audit the attribution model, which would reveal true revenue drivers.' Each strategy follows this 3-part pattern: (A) [keyword] + outcome sentence. (B) 'I [action verb] [specific approach] because [why it addresses the outcome].' (C) REQUIRED illustration — 'So if [specific trigger], I [specific action], which would [directional outcome].' CRITICAL: each strategy's 3 steps end at the NEXT [keyword]. NEVER add 'Beyond that,' or 'I also find that' at the END of a strategy block — these break the format. Last sentence of the final strategy names the overall business impact. No upfront listing. No adjectives. No adverbs. No 'utilize'. Never use vague pronouns.>\n\
Close: <One sentence the candidate says after the Answer. Mirrors the employer\'s specific business challenge from the system prompt — name the exact problem the employer is trying to solve (their growth constraint, market challenge, or operational goal) not a generic domain. Starts with 'That\'s why', 'This is why', 'The work I\'ve done in', or 'What I\'d bring here specifically is'. Max 20 words. End with a period. Never say 'this role', 'this work', 'this', 'it', or 'that'.>\n\
---\n\
Example: [1-2 word keyword] 3-5 word outcome title | <STAR story. 4 sentences maximum. All on ONE line. Each sentence starts with 'I'. Max 10 words per sentence. (1) Situation + Action combined — 'In [brief context], I [action verb] [specific approach].' (2) Optional second action — 'I also [action verb] [approach].' (3) Result — last sentence names the directional outcome achieved. NO inline 'So if' illustration. Draw only from candidate background. No invented metrics. Never use vague pronouns.>\n\
Ask: <2-4 word noun phrase naming what you're asking about — drawn directly from the topic the interviewer raised. e.g. 'attribution model', 'team prioritization', 'data maturity'> | <A genuine question the candidate asks the interviewer. Names a specific metric, tool, process, or concept related to what the interviewer asked about. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different angle, still related to the interviewer's question> | <A different genuine question. Names the specific topic — no vague pronouns. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I'm curious about'. Max 15 words.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like the company is focused on reducing customer acquisition cost through better targeting discipline.\n\
Solve: I've built paid search systems that directly reduce customer acquisition cost.\n\
Bridge: I'd approach that by starting with the data.\n\
Answer: [targeting]Customer acquisition cost rises when targeting drifts from intent. I audit keyword-to-landing-page alignment first because drift is the most common cost driver. So if a client's cost per acquisition rises, I audit match types and audience segments, which would bring conversion costs down within two weeks.[attribution]Attribution gaps hide which channels drive real revenue. I implement multi-touch attribution because single-touch models misallocate budget toward last click. So if return on ad spend drops without a clear cause, I audit the attribution model, which would reveal true revenue drivers and redirect budget to higher performers.\n\
Close: That's why my experience diagnosing targeting and attribution gaps maps directly to the growth constraints this team is working through.\n\
Example: cost per acquisition reduction | In a previous performance marketing role, I audited targeting drift on a high-spend account. I restructured match types and paused low-intent keywords. I also aligned landing pages to keyword intent. Cost per acquisition fell and conversion volume held steady.\n\
Ask: attribution model maturity | What attribution model does the team currently use across paid channels? | I ask because attribution gaps are often the first place I find budget misallocation.\n\
Ask: targeting review cadence | How frequently does the team audit keyword and audience targeting for drift? | I'm curious about whether the current review cadence catches drift before it compounds into cost.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Solve: Bridge: Answer: Close: Example: Ask: Ask:\n\
- Acknowledge: names business priority — never starts with 'I'. Never restates question.\n\
- Solve: starts with 'I'. Names specific business outcome from Acknowledge. Max 12 words.\n\
- Bridge: 5-8 words. Starts with 'I\'d' or 'I'. Never a question.\n\
- Answer: [keyword] strategy format. Each strategy = 3 steps: (1) [keyword] + outcome; (2) 'I [action] because [why]'; (3) 'So if [trigger], I [action], which would [outcome].' DO NOT skip step 3.\n\
- Answer: each strategy = 3 steps ending at the next [keyword]. NEVER put 'Beyond that,' or 'I also find that' at the END of a strategy block — place the next [keyword] immediately after step (C).\n\
- TONE: no adjectives or adverbs. No 'utilize'. Facts and actions only.\n\
- ALWAYS use 'I' — never 'we' or 'our team'.\n\
- ACRONYMS: write in full on first use, abbreviation in parentheses.\n\
- NEVER invent metrics, tools, skills, or details not explicitly in the candidate background.\n\
- NEVER name specific clients, employers, or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
Answer: <Technical reasoning on this same line. Each strategy MUST begin with a [1-2 word keyword] immediately before its opening sentence — no space between ] and first word. For each strategy: (A) [keyword] + one outcome sentence naming the technical principle or decision. (B) 'I [action verb] [specific approach] because [technical reason].' (C) REQUIRED illustration — 'So if [specific technical scenario], I [specific action], which would [directional outcome].' CRITICAL: each strategy's 3 steps end at the NEXT [keyword]. NEVER add 'Beyond that,' or 'I also' at the END of a strategy — start the next [keyword] immediately. Last sentence of the final strategy names the overall technical or business outcome. 2-3 strategies. No adjectives. No invented metrics. Draws from candidate background.>\n\
Close: <One sentence connecting the candidate\\'s technical approach to the employer\\'s specific challenge from the system prompt. Starts with 'That\\'s why', 'This is why', 'The work I\\'ve done in', or 'What I\\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase — the specific technical challenge or system named in the question> | <Question probing the technical depth of the problem — names the specific system, constraint, or scale involved. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different technical angle> | <A different question about tooling, architecture decisions, or technical tradeoffs the team faces. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like the challenge here is building a system that can scale without linearly increasing compute cost.\n\
Solve: I've architected distributed data pipelines that reduce processing cost at scale.\n\
Bridge: I'd tackle that by separating concerns first.\n\
Answer: [data partitioning]Cost at scale is driven by processing redundancy across all records equally. I partition data by access frequency first because uniform processing ignores how often records are actually read. So if a pipeline processes all records at the same rate, I segment by frequency tier, which would reduce compute cost on cold data without touching hot-path performance.[caching layer]A caching layer reduces repeat computation on frequently accessed results. I introduce a read-through cache at the service boundary because hot-path queries account for the majority of load in most systems. So if query latency rises with volume, I add caching between application and database, which would flatten cost growth and reduce database pressure.\n\
Close: That's why my experience building cost-aware pipelines maps directly to the scale challenge this team is working on.\n\
Ask: data access patterns | How does the team currently model access frequency when designing data pipelines? | I ask because partitioning by access pattern is often where the biggest cost savings come from.\n\
Ask: caching strategy | Does the team currently use a caching layer between the application and the data store? | I'm curious about where the team draws the line between cache complexity and query cost.\n\n\
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
- NEVER invent metrics, tools, skills, or details not in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
Close: <One sentence connecting the candidate\\'s collaboration approach to the employer\\'s specific team challenge from the system prompt. Starts with 'That\\'s why', 'This is why', 'The work I\\'ve done in', or 'What I\\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase — team collaboration dynamic or working norm> | <Question probing how the team collaborates or handles the specific dynamic the interviewer raised. Names the concrete process, cadence, or challenge. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — a different angle on team culture or feedback> | <A different question about how the team gives feedback, resolves disagreement, or works across functions. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like the team values collaboration that keeps cross-functional work moving without unnecessary process overhead.\n\
Solve: I've worked in environments where cross-functional alignment was the main bottleneck to on-time delivery.\n\
Bridge: I'd address that by building shared context early.\n\
Answer: In a previous cross-functional project, I mapped dependencies across three teams before the first sprint began. I ran a weekly 30-minute alignment call because async updates alone led to scope drift. So if a project spans multiple functions here, I can facilitate those alignment sessions, which would reduce mid-sprint blockers and keep delivery on track. I also documented decisions in a shared space because tribal knowledge slowed onboarding when team members changed. So if the team has high context-switching between projects, I can maintain a lightweight decision log, which would reduce time lost to re-explaining context.\n\
Close: That's why my approach to cross-functional alignment maps directly to the coordination challenges this team faces during growth.\n\
Ask: cross-functional alignment | How does the team currently manage alignment when multiple functions are contributing to a single delivery? | I ask because that coordination layer is where I've had the most impact in past roles.\n\
Ask: decision documentation | Does the team have a shared space for capturing key decisions and their rationale? | I'm curious about how the team preserves context when priorities shift between sprints.\n\n\
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
- NEVER invent metrics, tools, skills, or details not in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
        ctx_prefix, question
    )
}

fn build_character_prompt(ctx_prefix: &str, question: &str) -> String {
    format!(
        "{}The interviewer asked a personal character question: '{}'\n\n\
CRITICAL: This question is asking about personal qualities and how people outside of work perceive the candidate — NOT professional skills or job achievements. Output ONLY the exact labeled lines below. No preamble.\n\n\
Acknowledge: <One complete sentence naming what the interviewer is really trying to understand — insight into the candidate\\'s character, values, and how they show up in relationships. Opens with: 'It sounds like you want to understand', 'From your question, I can see you\\'re interested in', or 'It seems like the priority is understanding'. Completes with the character dimension being probed (e.g. reliability, self-awareness, interpersonal style). Max 20 words. Never starts with 'I'.>\n\
Solve: <One sentence naming 2-3 personal qualities the candidate\\'s friends or people close to them would genuinely say. These are CHARACTER traits — not job skills. e.g. directness, curiosity, reliability, warmth, follow-through, calm under pressure. Starts with 'My friends would say I am', 'People who know me well would describe me as', 'The people who know me well tend to say', or 'Outside of work, people would describe me as'. Draws from the candidate\\'s background — use their career narrative and the self-description in their LinkedIn About section or extra experience notes to infer authentic personal traits. Max 20 words.>\n\
Bridge: <One sentence and a brief personal example or story that illustrates these traits outside a work context — or at the intersection of personal and professional. Starts with 'For example,' or 'A good example of this is' or 'Outside of work,'. Names the specific trait in action. Max 20 words. Draws from background if available; otherwise use directional language.>\n\
Answer: <How these personal traits show up at work and create value for this specific employer. Each trait MUST begin with a [1-2 word keyword] immediately before its sentence — no space between ] and first word. For each trait: (A) [keyword] + one sentence naming how the personal trait translates to professional behaviour and business outcome. (B) One concrete proof point from the candidate background. Trait 2 onward: outcome sentence opens with 'Beyond that,' or 'I also'. Last sentence names the overall impact these traits have on teams and outcomes. 2-3 traits. No adjectives. No invented metrics. Draws from candidate background.>\n\
Close: <One sentence connecting the candidate\\'s personal qualities to what the employer needs from their team, as described in the system prompt. Starts with 'That\\'s why', 'This is why', 'The work I\\'ve done in', or 'What I\\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase about team culture or values — what kind of person thrives here> | <Question about what personal or interpersonal qualities tend to make people successful at this company or in this team. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — team dynamics or working style> | <A different question about how the team works together or what values shape day-to-day interactions. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: It sounds like you want to understand what kind of person I am outside of the professional context.\n\
Solve: My friends would say I am direct, curious, and someone who follows through on commitments.\n\
Bridge: For example, when I took on a personal project learning data analysis, I finished every module and applied it to a real problem.\n\
Answer: [directness]Directness at work means stakeholders always know where a project stands. I surface blockers early because delayed transparency creates larger downstream problems. Beyond that, I also bring[curiosity]genuine curiosity to problems I haven't seen before. I've pursued side projects that directly informed how I approach analytical challenges in my work. So if the team values people who learn independently, I can bring new techniques into the workflow, which would keep the team's approach current.[follow-through]Follow-through means every commitment closes cleanly. I track open items weekly because incomplete loops slow down everyone who depends on my output. So if reliability on deliverables matters here, I can be counted on to close without reminders, which would reduce the management overhead the team carries.\n\
Close: That's why the qualities people see in me outside of work show up the same way in how I operate within a team.\n\
Ask: success traits | What personal qualities tend to make people successful on this team long-term? | I ask because I want to understand whether the way I naturally operate fits how this team works.\n\
Ask: team working style | How would you describe the day-to-day interaction style on this team? | I'm curious about whether the team tends to work more independently or collaboratively on a given day.\n\n\
Rules:\n\
- Output ONLY these lines. No extra text.\n\
- Acknowledge: names the character dimension, not the job skill. Never starts with 'I'.\n\
- Solve: names PERSONAL traits only — not professional competencies. e.g. 'direct', 'reliable', 'curious', 'calm', 'empathetic'. Starts with 'My friends would say', 'People who know me well would describe me as', 'The people who know me well tend to say', or 'Outside of work, people would describe me as'.\n\
- Bridge: a specific personal example — not a work achievement. Draws from background if available. NEVER invent personal examples.\n\
- Answer text must be on the same line as 'Answer:'. Uses [keyword] format. Each trait = personal quality translated to professional value.\n\
- Answer: draws ONLY from candidate background. NEVER invent traits, metrics, or personal details not in the background.\n\
- Close: connects personal traits to the specific team or employer challenge from the system prompt.\n\
- Always use 'I' — never 'we' or 'our'.\n\
- Acronyms: write in full on first use.\n\
- NEVER invent metrics, traits, or details not in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
Close: <One sentence connecting BOTH dimensions to the employer\\'s specific business challenge from the system prompt. Starts with \\'That\\'s why\\', \\'This is why\\', \\'The work I\\'ve done in\\', or \\'What I\\'d bring here specifically is\\'. Max 20 words. Never say \\'this role\\', \\'this\\', \\'it\\'.>\n\
---\n\
Ask: <2-4 word noun phrase at the intersection of {topic1} and {topic2} — names the bridging concept, not either standalone topic. e.g. \\'growth path alignment\\', \\'background fit timeline\\'> | <Question that naturally bridges BOTH dimensions — probes the specific connection between candidate background AND this role\\'s challenge. Names the concrete metric, process, or domain. Ends with \\'?\\'.> | <1 sentence. Starts with \\'I ask because\\' or \\'I\\'m curious about\\'. Max 15 words.>\n\
Ask: <2-4 word noun phrase — a different bridging angle, still connecting both dimensions> | <A different question probing the intersection. Names the specific metric, process, or domain where both aspects join. Ends with \\'?\\'.> | <1 sentence. Starts with \\'I ask because\\' or \\'I\\'m curious about\\'. Max 15 words.>\n\n\
Rules:\n\
- Output ONLY: Acknowledge:, Answer:, Close:, then two Ask: lines. No other labels. No preamble.\n\
- Answer addresses {topic1} first, transitions once, then addresses {topic2}. Both parts draw ONLY from candidate background in the system prompt.\n\
- NEVER invent skills, tools, metrics, or details not in the candidate background.\n\
- Ask topics probe the INTERSECTION of both dimensions — not topics from either standalone answer.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.\n\
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
Impact: <One sentence naming a specific professional moment — a project delay, a relationship that needed repair, a decision that cost momentum — where this weakness had a real, tangible consequence. Tactful: no blame, no drama, no names. Starts with 'There was a point', 'Earlier in my career', or 'I'. Max 18 words. Draws from the candidate background in the system prompt.>\n\
Solve: <One sentence. The specific behaviour change or structured approach the candidate adopted to address this limitation. Starts with 'I' or 'I\\'ve'. Max 15 words. Names the concrete action — not 'I am working on it'. Draws from ALL candidate context in the system prompt: CV, LinkedIn, portfolio, extra experience notes, early career context.>\n\
Bridge: <One sentence of concrete evidence from the candidate background that this approach has produced improvement. Starts with 'I'. Names the specific project, role, or context and the directional outcome — no invented metrics. Max 15 words. If no direct evidence exists in the background, use directional language: 'Since then, I have measurably improved' — never fabricate details.>\n\
Answer: <One sentence redirecting to a compensating strength that is directly relevant to the employer's specific business challenge from the system prompt (job description, company information, interviewer context). Uses [keyword] strategy format: [1-2 word keyword] + one sentence naming the strength and its business outcome for this employer. Then one concrete proof point from the candidate background (CV, LinkedIn, portfolio, extra experience). Then a directional outcome for this specific employer. No adjectives. No invented metrics.>\n\
Close: <One sentence connecting the candidate's growth trajectory to the employer's specific challenge from the system prompt. Starts with 'That growth is exactly why', 'That\\'s exactly why', or 'The work I\\'ve done on'. Max 20 words. References the employer\\'s actual business problem — not a generic domain. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase related to team growth or development culture> | <Question showing the candidate is actively developing — probes how the team or company supports growth in this specific area. Names a concrete process, tool, or domain. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — different angle on development or feedback> | <Question about how the team gives feedback or how performance is measured in this area. Names a specific metric or process. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: One area I have actively worked on is presenting strategic context before going into channel-level detail in stakeholder settings.\n\
Impact: There was a point where a campaign review with a commercial director focused on budget cuts rather than strategy, because I led with channel metrics instead of pipeline impact.\n\
Solve: I've since adopted a structure where I open every performance review with pipeline contribution and cost per opportunity before going into channel detail.\n\
Bridge: I've measurably improved my ability to hold those conversations — senior stakeholders now engage on strategy rather than redirecting to cost reduction.\n\
Answer: [performance storytelling]Attribution clarity drives confident investment decisions. I've built pipeline-linked reporting frameworks at a B2B software company that translated paid channel data into language sales and finance teams could act on, which meant budget decisions were made faster. For a team scaling acquisition, this means paid media earns its budget through revenue language, not channel jargon.\n\
Close: That growth is exactly why I'm focused on roles where paid acquisition reports to pipeline and revenue, not just channel KPIs.\n\
Ask: stakeholder reporting format | How does the team currently present paid performance to commercial leadership — outcome-first or metrics-first? | I ask because the reporting format shapes whether paid media is seen as a strategic function or a cost centre.\n\
Ask: performance feedback process | How does the team give feedback when communication to non-technical stakeholders needs to improve? | I'm curious because it's the area I continue to actively develop.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Impact: Solve: Bridge: Answer: Close: Ask: Ask:\n\
- Acknowledge: names a REAL, SPECIFIC limitation — never 'perfectionist' or 'work too hard'. Draws ONLY from candidate background — NEVER invent a weakness.\n\
- Solve: concrete action taken, not vague aspiration. Draws from ALL candidate context.\n\
- Bridge: real evidence from background. If none, directional language — NEVER fabricate metric or outcome.\n\
- Answer: [keyword] compensating strength MUST be directly relevant to employer's specific challenge from system prompt. NEVER invent skills.\n\
- TONE: no adjectives, no 'passionate'. Facts only.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- ACRONYMS: write in full on first use, abbreviation in parentheses.\n\
- NEVER invent metrics, percentages, tools, or details not in background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
Close: <One sentence connecting the candidate\\'s reasoning approach to the employer\\'s specific business challenge from the system prompt. Draws from the job description and company information. Starts with 'That\\'s why', 'This is why', 'The work I\\'ve done in', or 'What I\\'d bring here specifically is'. Max 20 words. Never say 'this role', 'this', 'it'.>\n\
---\n\
Ask: <2-4 word noun phrase probing the real context behind the hypothetical> | <Question that surfaces the actual business situation the interviewer had in mind. Names a specific metric, process, or recent event. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\
Ask: <2-4 word noun phrase — different angle on how success is measured or what\\'s been tried> | <Question about how the team has approached this challenge previously or how success would be measured. Names a concrete metric or process. Ends with '?'.> | <1 sentence — what YOU say if the interviewer asks why YOU are asking THEM this question. Starts with 'I ask because' or 'I\\'m curious about'. Ties to your career direction or this employer's specific challenge — NEVER generic ('I want to understand the team'). Max 15 words. Must be a complete sentence.>\n\n\
EXAMPLE OUTPUT (adapt every detail to the actual candidate background — structure is fixed):\n\
Acknowledge: From your question, the concern seems to be maintaining campaign effectiveness when the brief or objectives are unclear from the start.\n\
Solve: I'd start by separating what is unclear from what is fixed — the constraints that exist regardless of scope.\n\
Bridge: I'd then work through each layer.\n\
Answer: [scope clarification]Unclear scope is a stakeholder alignment gap, not a campaign execution problem. I clarify the success metric and non-negotiable constraints before changing any live campaigns, because ambiguous objectives produce rework rather than results. So if a brief arrives with conflicting objectives, I resolve the objective definition first, which prevents spend on outcomes that are already going to change.[priority sequencing]Once the primary objective is clear, I sequence optimisation tasks by impact on that objective, not by urgency. I prioritise the actions that protect pipeline quality before volume or efficiency, because quality degradation is harder to reverse than volume loss. So if the timeline compresses, I would concentrate remaining budget on the highest-converting audience segments, which would maintain pipeline quality even with lower total spend.\n\
Close: The work I've done navigating ambiguous briefs in performance accounts is exactly what allows this kind of uncertainty to be handled without losing pipeline contribution here.\n\
Ask: scope ambiguity frequency | How often do campaign briefs arrive with unclear or conflicting objectives, and what typically causes it? | I ask because the frequency determines how much upfront alignment process I need to build into the workflow.\n\
Ask: success criteria ownership | Who has the final say on what a successful campaign outcome looks like — marketing, sales, or both? | I'm curious about the decision authority because clear ownership is what makes brief resolution fast.\n\n\
Rules:\n\
- LABELS in order: Acknowledge: Solve: Bridge: Answer: Close: Ask: Ask:\n\
- Answer: candidate reasons through the hypothetical — does NOT recall a story as the main answer.\n\
- Answer: each strategy MUST complete all 3 steps (keyword + outcome; action + why; illustration) before moving to next.\n\
- Answer: draws from ALL candidate context: CV, LinkedIn, portfolio, extra experience notes, early career context. NEVER invent approach, tools, or skills.\n\
- TONE: no adjectives. No invented metrics. Directional language only.\n\
- ALWAYS use 'I' — never 'we' or 'our'.\n\
- ACRONYMS: write in full on first use, abbreviation in parentheses.\n\
- NEVER invent metrics, tools, skills, or details not in the candidate background.\n\
- NEVER name specific clients or companies — refer by industry only.\n\
- Ask noun phrase: names the underlying business concept — NEVER copies words from the interviewer's question.\n\
- Ask follow-up (3rd pipe): what YOU say if the interviewer asks why YOU are asking THEM — not a response to the original question.",
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
    fn company_research_uses_core_offering_labels() {
        let p = build_user_prompt("What do you know about our company?", &[]);
        assert!(p.contains("CoreOffering:"));
        assert!(p.contains("Positioning:"));
        assert!(p.contains("Segment:"));
        assert!(p.contains("Growth:"));
        assert!(p.contains("Backers:"));
        assert!(p.contains("Enthusiasm:"));
    }

    #[test]
    fn company_research_classified_correctly() {
        assert!(matches!(
            classify_question("What do you know about us?").0,
            QuestionType::CompanyResearch
        ));
        assert!(matches!(
            classify_question("Have you heard about our company?").0,
            QuestionType::CompanyResearch
        ));
        assert!(matches!(
            classify_question("How familiar are you with what we do?").0,
            QuestionType::CompanyResearch
        ));
    }

    #[test]
    fn company_research_beats_motivation_for_company_knowledge() {
        // "what do you know about us" is company_research, not motivation
        assert!(matches!(
            classify_question("What do you know about our company and why are you excited about us?").0,
            QuestionType::CompanyResearch
        ));
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
