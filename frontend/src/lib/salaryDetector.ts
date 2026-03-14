const SALARY_TRIGGERS = [
  'salary', 'compensation', 'pay', 'package', 'benefits', 'equity', 'stock',
  'bonus', 'how much', 'what are you looking', 'what do you expect',
  'current salary', 'current comp', 'previous salary', 'your range',
  'budget for this role', 'offer', 'total comp',
];

export function isSalaryQuestion(text: string): boolean {
  const lower = text.toLowerCase();
  return SALARY_TRIGGERS.some(t => lower.includes(t));
}
