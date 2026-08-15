use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::model::{
    ArchIssue, ConstraintResult, MutationReport, RequirementArchReport, SpecificationStatus,
};
use crate::{Result, SpecError, reference};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StructuredClaim {
    pub quantifier: String,
    pub subject: String,
    pub relation: String,
    pub object: String,
}

impl StructuredClaim {
    fn parse(value: &str, line: usize) -> Result<Self> {
        let parts = value.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(SpecError::new(format!(
                "requirement registry:{line}: @claim requires quantifier, subject, relation, and object",
            )));
        }
        Ok(Self {
            quantifier: parts[0].to_string(),
            subject: parts[1].to_string(),
            relation: parts[2].to_string(),
            object: parts[3].to_string(),
        })
    }

    pub(crate) fn gloss(&self) -> String {
        let quantifier = match self.quantifier.as_str() {
            "all" => "Every".to_string(),
            "any" => "At least one".to_string(),
            other => title_words(other),
        };
        format!(
            "{quantifier} {} must {} {}.",
            words(&self.subject),
            words(&self.relation),
            words(&self.object),
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProfileRule {
    pub profile: String,
    pub expectation: String,
}

#[derive(Clone, Debug)]
pub(crate) struct EvidenceRule {
    pub provider: String,
    pub schema: String,
    pub minimum_grade: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ExampleCase {
    pub name: String,
    pub contract: String,
    pub expected: String,
    pub bindings: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(crate) struct MutationCase {
    pub name: String,
    pub contract: String,
    pub severity: String,
    pub expression: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Requirement {
    pub id: String,
    pub title: String,
    pub level: String,
    pub area: String,
    pub normative: String,
    pub claim: StructuredClaim,
    pub contract_claim: StructuredClaim,
    pub terms: Vec<String>,
    pub contracts: Vec<String>,
    pub profiles: Vec<ProfileRule>,
    pub evidence: Vec<EvidenceRule>,
    pub examples: Vec<ExampleCase>,
    pub counterexamples: Vec<ExampleCase>,
    pub outside_scope: Vec<String>,
    pub mutations: Vec<MutationCase>,
    pub ratified_hash: String,
    pub reviewer: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RequirementRegistry {
    pub contract_profile: String,
    pub glossary: BTreeSet<String>,
    pub requirements: Vec<Requirement>,
}

#[derive(Default)]
struct RequirementBuilder {
    id: String,
    title: String,
    level: String,
    area: String,
    normative: String,
    claim: Option<StructuredClaim>,
    contract_claim: Option<StructuredClaim>,
    terms: Vec<String>,
    contracts: Vec<String>,
    profiles: Vec<ProfileRule>,
    evidence: Vec<EvidenceRule>,
    examples: Vec<ExampleCase>,
    counterexamples: Vec<ExampleCase>,
    outside_scope: Vec<String>,
    mutations: Vec<MutationCase>,
    ratified_hash: String,
    reviewer: String,
}

impl RequirementBuilder {
    fn finish(self, line: usize) -> Result<Requirement> {
        let requirement_id = self.id.clone();
        let required = |value: String, name: &str| {
            if value.is_empty() {
                Err(SpecError::new(format!(
                    "requirement registry:{line}: requirement {} has no @{name}",
                    requirement_id
                )))
            } else {
                Ok(value)
            }
        };
        Ok(Requirement {
            id: required(self.id, "requirement-id")?,
            title: required(self.title, "title")?,
            level: required(self.level, "level")?,
            area: required(self.area, "area")?,
            normative: required(self.normative, "normative")?,
            claim: self.claim.ok_or_else(|| {
                SpecError::new(format!("requirement registry:{line}: missing @claim"))
            })?,
            contract_claim: self.contract_claim.ok_or_else(|| {
                SpecError::new(format!(
                    "requirement registry:{line}: missing @contract-claim"
                ))
            })?,
            terms: self.terms,
            contracts: self.contracts,
            profiles: self.profiles,
            evidence: self.evidence,
            examples: self.examples,
            counterexamples: self.counterexamples,
            outside_scope: self.outside_scope,
            mutations: self.mutations,
            ratified_hash: required(self.ratified_hash, "ratification")?,
            reviewer: required(self.reviewer, "ratification reviewer")?,
        })
    }
}

impl RequirementRegistry {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(|error| {
            SpecError::new(format!(
                "could not read requirement registry {}: {error}",
                path.display()
            ))
        })?;
        Self::parse(&source)
    }

    fn parse(source: &str) -> Result<Self> {
        let mut contract_profile = None;
        let mut glossary = BTreeSet::new();
        let mut requirements = Vec::new();
        let mut current: Option<RequirementBuilder> = None;

        for (index, raw_line) in source.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw_line.trim();
            let Some(annotation) = trimmed.strip_prefix("-- @") else {
                continue;
            };
            let (key, value) = annotation
                .split_once(' ')
                .map(|(key, value)| (key, value.trim()))
                .unwrap_or((annotation, ""));
            if key == "contract-profile" {
                contract_profile = Some(value.to_string());
                continue;
            }
            if key == "glossary" {
                glossary.insert(value.to_string());
                continue;
            }
            if key == "requirement" {
                // This annotation binds an executable integrity declaration.
                // It is consumed by the contract evaluator rather than the
                // static requirement registry.
                continue;
            }
            if key == "requirement-id" {
                if let Some(builder) = current.take() {
                    requirements.push(builder.finish(line)?);
                }
                current = Some(RequirementBuilder {
                    id: value.to_string(),
                    ..RequirementBuilder::default()
                });
                continue;
            }
            if key == "end-requirement" {
                let builder = current.take().ok_or_else(|| {
                    SpecError::new(format!(
                        "requirement registry:{line}: @end-requirement without a requirement",
                    ))
                })?;
                requirements.push(builder.finish(line)?);
                continue;
            }
            let builder = current.as_mut().ok_or_else(|| {
                SpecError::new(format!(
                    "requirement registry:{line}: @{key} is outside a requirement",
                ))
            })?;
            match key {
                "title" => builder.title = value.to_string(),
                "level" => builder.level = value.to_string(),
                "area" => builder.area = value.to_string(),
                "normative" => builder.normative = value.to_string(),
                "claim" => builder.claim = Some(StructuredClaim::parse(value, line)?),
                "contract-claim" => {
                    builder.contract_claim = Some(StructuredClaim::parse(value, line)?)
                }
                "term" => builder.terms.push(value.to_string()),
                "contract" => builder.contracts.push(value.to_string()),
                "profile" => builder.profiles.push(parse_profile(value, line)?),
                "evidence" => builder.evidence.push(parse_evidence(value, line)?),
                "example" => builder.examples.push(parse_example(value, line, "pass")?),
                "counterexample" => builder
                    .counterexamples
                    .push(parse_example(value, line, "fail")?),
                "outside-scope" => builder.outside_scope.push(value.to_string()),
                "mutation" => builder.mutations.push(parse_mutation(value, line)?),
                "ratification" => {
                    let mut fields = value.split_whitespace();
                    builder.ratified_hash = fields.next().unwrap_or_default().to_string();
                    builder.reviewer = fields.next().unwrap_or_default().to_string();
                }
                _ => {
                    return Err(SpecError::new(format!(
                        "requirement registry:{line}: unknown annotation @{key}",
                    )));
                }
            }
        }
        if let Some(builder) = current {
            requirements.push(builder.finish(source.lines().count())?);
        }
        let contract_profile = contract_profile.ok_or_else(|| {
            SpecError::new("requirement registry has no @contract-profile annotation")
        })?;
        if requirements.len() < 5 {
            return Err(SpecError::new(format!(
                "requirement registry has {} requirements; the demonstrator requires at least five",
                requirements.len()
            )));
        }
        let mut ids = BTreeSet::new();
        for requirement in &requirements {
            if !ids.insert(requirement.id.clone()) {
                return Err(SpecError::new(format!(
                    "requirement registry defines {} more than once",
                    requirement.id
                )));
            }
        }
        Ok(Self {
            contract_profile,
            glossary,
            requirements,
        })
    }
}

fn parse_profile(value: &str, line: usize) -> Result<ProfileRule> {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 2 || !matches!(fields[1], "must-pass" | "may-reject" | "not-applicable") {
        return Err(SpecError::new(format!(
            "requirement registry:{line}: @profile requires PROFILE and must-pass, may-reject, or not-applicable",
        )));
    }
    Ok(ProfileRule {
        profile: fields[0].to_string(),
        expectation: fields[1].to_string(),
    })
}

fn parse_evidence(value: &str, line: usize) -> Result<EvidenceRule> {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 3 {
        return Err(SpecError::new(format!(
            "requirement registry:{line}: @evidence requires PROVIDER, SCHEMA, and minimum grade",
        )));
    }
    Ok(EvidenceRule {
        provider: fields[0].to_string(),
        schema: fields[1].to_string(),
        minimum_grade: fields[2].to_string(),
    })
}

fn parse_example(value: &str, line: usize, expected: &str) -> Result<ExampleCase> {
    let mut fields = value.splitn(3, ' ');
    let name = fields.next().unwrap_or_default();
    let contract = fields.next().unwrap_or_default();
    let bindings_source = fields.next().unwrap_or_default();
    if name.is_empty() || contract.is_empty() || bindings_source.is_empty() {
        return Err(SpecError::new(format!(
            "requirement registry:{line}: example requires NAME, CONTRACT, and bindings",
        )));
    }
    let mut bindings = BTreeMap::new();
    for binding in bindings_source.split(';') {
        let (binding_name, expression) = binding.split_once('=').ok_or_else(|| {
            SpecError::new(format!(
                "requirement registry:{line}: malformed example binding `{binding}`",
            ))
        })?;
        bindings.insert(
            binding_name.trim().to_string(),
            expression.trim().to_string(),
        );
    }
    Ok(ExampleCase {
        name: name.to_string(),
        contract: contract.to_string(),
        expected: expected.to_string(),
        bindings,
    })
}

fn parse_mutation(value: &str, line: usize) -> Result<MutationCase> {
    let mut fields = value.splitn(4, ' ');
    let name = fields.next().unwrap_or_default();
    let contract = fields.next().unwrap_or_default();
    let severity = fields.next().unwrap_or_default();
    let expression = fields.next().unwrap_or_default();
    if name.is_empty() || contract.is_empty() || severity.is_empty() || expression.is_empty() {
        return Err(SpecError::new(format!(
            "requirement registry:{line}: mutation requires NAME, CONTRACT, SEVERITY, and expression",
        )));
    }
    Ok(MutationCase {
        name: name.to_string(),
        contract: contract.to_string(),
        severity: severity.to_string(),
        expression: expression.to_string(),
    })
}

pub(crate) fn evaluate_arch(
    registry: &RequirementRegistry,
    constraints: &[ConstraintResult],
) -> Result<Vec<RequirementArchReport>> {
    let constraint_map = constraints
        .iter()
        .map(|constraint| (constraint.name.as_str(), constraint))
        .collect::<BTreeMap<_, _>>();
    let mut reports = Vec::new();
    for requirement in &registry.requirements {
        let mut issues = Vec::new();
        if requirement.claim != requirement.contract_claim {
            issues.push(issue(
                "PROSE_CONTRACT_MISMATCH",
                format!(
                    "structured prose claim differs from executable contract claim; generated gloss is {:?}",
                    requirement.contract_claim.gloss()
                ),
            ));
        }
        for term in &requirement.terms {
            if !registry.glossary.contains(term) {
                issues.push(issue(
                    "TERM_UNRESOLVED",
                    format!("term `{term}` is absent from the glossary"),
                ));
            }
        }
        if requirement.contracts.is_empty() {
            issues.push(issue("UNFORMALIZED", "no executable contract is bound"));
        }
        for contract_name in &requirement.contracts {
            match constraint_map.get(contract_name.as_str()) {
                Some(constraint) if constraint.requirement == requirement.id => {}
                Some(constraint) => issues.push(issue(
                    "CONTRACT_BINDING_MISMATCH",
                    format!(
                        "contract `{contract_name}` is annotated {}, not {}",
                        constraint.requirement, requirement.id
                    ),
                )),
                None => issues.push(issue(
                    "CONTRACT_MISSING",
                    format!("contract `{contract_name}` was not found"),
                )),
            }
        }
        if requirement.examples.is_empty() {
            issues.push(issue("EXAMPLE_MISSING", "no positive example is bound"));
        }
        if requirement.counterexamples.is_empty() {
            issues.push(issue(
                "COUNTEREXAMPLE_MISSING",
                "no counterexample is bound",
            ));
        }
        if requirement.outside_scope.is_empty() {
            issues.push(issue(
                "OUTSIDE_SCOPE_MISSING",
                "no outside-scope example is bound",
            ));
        }
        evaluate_examples(requirement, &constraint_map, &mut issues)?;
        let mutations = evaluate_mutations(requirement)?;
        for mutation in &mutations {
            if mutation.severity == "critical" && !mutation.detected {
                issues.push(issue(
                    "CRITICAL_MUTATION_SURVIVED",
                    format!("critical mutation `{}` was not detected", mutation.name),
                ));
            }
        }
        let current_hash = arch_hash(requirement, &constraint_map);
        if current_hash != requirement.ratified_hash {
            issues.push(issue(
                "RATIFICATION_STALE",
                format!(
                    "ratified hash {} does not match current arch hash {current_hash}",
                    requirement.ratified_hash
                ),
            ));
        }
        let has_non_ratification_issue =
            issues.iter().any(|item| item.code != "RATIFICATION_STALE");
        let status = if has_non_ratification_issue {
            if issues
                .iter()
                .any(|item| item.code == "CRITICAL_MUTATION_SURVIVED")
                && issues.iter().all(|item| {
                    matches!(
                        item.code.as_str(),
                        "CRITICAL_MUTATION_SURVIVED" | "RATIFICATION_STALE"
                    )
                })
            {
                SpecificationStatus::MutationSurvived
            } else {
                SpecificationStatus::SpecInvalid
            }
        } else if issues.iter().any(|item| item.code == "RATIFICATION_STALE") {
            SpecificationStatus::ReviewRequired
        } else {
            SpecificationStatus::Ratified
        };
        reports.push(RequirementArchReport {
            requirement: requirement.id.clone(),
            title: requirement.title.clone(),
            level: requirement.level.clone(),
            area: requirement.area.clone(),
            normative_prose: requirement.normative.clone(),
            generated_gloss: requirement.contract_claim.gloss(),
            current_arch_hash: current_hash,
            ratified_arch_hash: requirement.ratified_hash.clone(),
            reviewer: requirement.reviewer.clone(),
            status,
            issues,
            mutations,
        });
    }
    Ok(reports)
}

fn evaluate_examples(
    requirement: &Requirement,
    constraints: &BTreeMap<&str, &ConstraintResult>,
    issues: &mut Vec<ArchIssue>,
) -> Result<()> {
    for example in requirement
        .examples
        .iter()
        .chain(requirement.counterexamples.iter())
    {
        let Some(constraint) = constraints.get(example.contract.as_str()) else {
            continue;
        };
        let actual = reference::evaluate_case(&constraint.expression, &example.bindings)?;
        let expected = example.expected == "pass";
        if actual != expected {
            issues.push(issue(
                if expected {
                    "EXAMPLE_CONFLICT"
                } else {
                    "COUNTEREXAMPLE_CONFLICT"
                },
                format!(
                    "example `{}` expected {} but contract `{}` returned {}",
                    example.name, example.expected, example.contract, actual
                ),
            ));
        }
    }
    Ok(())
}

fn evaluate_mutations(requirement: &Requirement) -> Result<Vec<MutationReport>> {
    requirement
        .mutations
        .iter()
        .map(|mutation| {
            let relevant = requirement
                .examples
                .iter()
                .chain(requirement.counterexamples.iter())
                .filter(|example| example.contract == mutation.contract)
                .collect::<Vec<_>>();
            let mut detected = false;
            for example in relevant {
                let actual = reference::evaluate_case(&mutation.expression, &example.bindings)?;
                let expected = example.expected == "pass";
                detected |= actual != expected;
            }
            Ok(MutationReport {
                name: mutation.name.clone(),
                contract: mutation.contract.clone(),
                severity: mutation.severity.clone(),
                expression: mutation.expression.clone(),
                detected,
            })
        })
        .collect()
}

fn arch_hash(requirement: &Requirement, constraints: &BTreeMap<&str, &ConstraintResult>) -> String {
    let mut canonical = vec![
        format!("id={}", requirement.id),
        format!("title={}", requirement.title),
        format!("level={}", requirement.level),
        format!("area={}", requirement.area),
        format!("normative={}", requirement.normative),
        format!("claim={:?}", requirement.claim),
        format!("contract-claim={:?}", requirement.contract_claim),
    ];
    canonical.extend(
        requirement
            .terms
            .iter()
            .map(|value| format!("term={value}")),
    );
    for contract in &requirement.contracts {
        let expression = constraints
            .get(contract.as_str())
            .map(|constraint| constraint.expression.as_str())
            .unwrap_or("<missing>");
        canonical.push(format!("contract={contract}:{expression}"));
    }
    for profile in &requirement.profiles {
        canonical.push(format!(
            "profile={}:{}",
            profile.profile, profile.expectation
        ));
    }
    for evidence in &requirement.evidence {
        canonical.push(format!(
            "evidence={}:{}:{}",
            evidence.provider, evidence.schema, evidence.minimum_grade
        ));
    }
    for example in requirement
        .examples
        .iter()
        .chain(requirement.counterexamples.iter())
    {
        canonical.push(format!(
            "example={}:{}:{}:{:?}",
            example.name, example.contract, example.expected, example.bindings
        ));
    }
    canonical.extend(
        requirement
            .outside_scope
            .iter()
            .map(|value| format!("outside={value}")),
    );
    for mutation in &requirement.mutations {
        canonical.push(format!(
            "mutation={}:{}:{}:{}",
            mutation.name, mutation.contract, mutation.severity, mutation.expression
        ));
    }
    hash_bytes(canonical.join("\n").as_bytes())
}

pub(crate) fn specification_version(reports: &[RequirementArchReport]) -> String {
    let source = reports
        .iter()
        .map(|report| format!("{}={}", report.requirement, report.current_arch_hash))
        .collect::<Vec<_>>()
        .join("\n");
    hash_bytes(source.as_bytes())
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn issue(code: &str, detail: impl Into<String>) -> ArchIssue {
    ArchIssue {
        code: code.to_string(),
        detail: detail.into(),
    }
}

fn words(value: &str) -> String {
    value.replace('-', " ")
}

fn title_words(value: &str) -> String {
    let mut words = words(value);
    if let Some(first) = words.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_a_deterministic_contract_gloss() {
        let claim =
            StructuredClaim::parse("all aborted-transaction preserve committed-state", 1).unwrap();
        assert_eq!(
            claim.gloss(),
            "Every aborted transaction must preserve committed state."
        );
    }
}
