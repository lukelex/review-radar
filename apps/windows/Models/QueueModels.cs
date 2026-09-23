using System.Text.Json.Serialization;

namespace ReviewRadar.Windows.Models;

public sealed class QueueResponse
{
    [JsonPropertyName("schemaVersion")]
    public int SchemaVersion { get; init; }

    [JsonPropertyName("capturedAt")]
    public string CapturedAt { get; init; } = "";

    [JsonPropertyName("view")]
    public string View { get; init; } = "";

    [JsonPropertyName("ranking")]
    public string Ranking { get; init; } = "";

    [JsonPropertyName("count")]
    public int Count { get; init; }

    [JsonPropertyName("sourceCount")]
    public int SourceCount { get; init; }

    [JsonPropertyName("suppressedCount")]
    public int SuppressedCount { get; init; }

    [JsonPropertyName("notificationEligibleIds")]
    public IReadOnlyList<string> NotificationEligibleIds { get; init; } = [];

    [JsonPropertyName("pullRequests")]
    public IReadOnlyList<PullRequestCard> PullRequests { get; init; } = [];
}

public sealed class PullRequestCard
{
    [JsonPropertyName("id")]
    public string Id { get; init; } = "";

    [JsonPropertyName("repository")]
    public string Repository { get; init; } = "";

    [JsonPropertyName("number")]
    public int Number { get; init; }

    [JsonPropertyName("title")]
    public string Title { get; init; } = "";

    [JsonPropertyName("url")]
    public string Url { get; init; } = "";

    [JsonPropertyName("updatedAt")]
    public string UpdatedAt { get; init; } = "";

    [JsonPropertyName("lifecycle")]
    public string Lifecycle { get; init; } = "";

    [JsonPropertyName("priority")]
    public string Priority { get; init; } = "";

    [JsonPropertyName("relationship")]
    public string Relationship { get; init; } = "";

    [JsonPropertyName("action")]
    public string Action { get; init; } = "";

    [JsonPropertyName("actionLabel")]
    public string ActionLabel { get; init; } = "";

    [JsonPropertyName("attentionRequired")]
    public bool AttentionRequired { get; init; }

    [JsonPropertyName("explanation")]
    public Explanation Explanation { get; init; } = new();

    [JsonPropertyName("currentFingerprint")]
    public string CurrentFingerprint { get; init; } = "";

    public string RepositoryAndNumber => $"{Repository} #{Number}";

    public string PrimaryReasonSummary => Explanation.Reasons.FirstOrDefault()?.Summary ?? Explanation.Heading;
}

public sealed class Explanation
{
    [JsonPropertyName("heading")]
    public string Heading { get; init; } = "";

    [JsonPropertyName("reasons")]
    public IReadOnlyList<Reason> Reasons { get; init; } = [];
}

public sealed class Reason
{
    [JsonPropertyName("code")]
    public string Code { get; init; } = "";

    [JsonPropertyName("summary")]
    public string Summary { get; init; } = "";

    [JsonPropertyName("evidence")]
    public IReadOnlyList<string> Evidence { get; init; } = [];

    [JsonPropertyName("nextAction")]
    public NextAction NextAction { get; init; } = new();
}

public sealed class NextAction
{
    [JsonPropertyName("label")]
    public string Label { get; init; } = "";

    [JsonPropertyName("url")]
    public string Url { get; init; } = "";
}
