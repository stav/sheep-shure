export interface Client {
  id: string;
  first_name: string;
  last_name: string;
  middle_name?: string;
  dob?: string;
  gender?: string;
  phone?: string;
  phone2?: string;
  email?: string;
  address_line1?: string;
  address_line2?: string;
  city?: string;
  state?: string;
  zip?: string;
  county?: string;
  mbi?: string;
  part_a_date?: string;
  part_b_date?: string;
  orec?: string;
  esrd_status: boolean;
  is_dual_eligible: boolean;
  dual_status_code?: string;
  lis_level?: string;
  medicaid_id?: string;
  lead_source?: string;
  original_effective_date?: string;
  is_active: boolean;
  tags?: string[];
  created_at: string;
  updated_at: string;
}

export interface ClientListItem {
  id: string;
  first_name: string;
  last_name: string;
  dob?: string;
  phone?: string;
  email?: string;
  city?: string;
  state?: string;
  zip?: string;
  mbi?: string;
  is_active: boolean;
  is_dual_eligible: boolean;
}

export interface ClientFilters {
  search?: string;
  carrier_id?: string;
  plan_type_code?: string;
  status_code?: string;
  state?: string;
  zip?: string;
  is_dual_eligible?: boolean;
  is_active?: boolean;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
}

export interface Enrollment {
  id: string;
  client_id: string;
  plan_id?: string;
  carrier_id?: string;
  plan_type_code?: string;
  plan_name?: string;
  contract_number?: string;
  pbp_number?: string;
  effective_date?: string;
  termination_date?: string;
  application_date?: string;
  status_code: string;
  enrollment_period?: string;
  disenrollment_reason?: string;
  premium?: number;
  confirmation_number?: string;
  enrollment_source?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface Carrier {
  id: string;
  name: string;
  short_name?: string;
  is_active: boolean;
}

export interface DashboardStats {
  total_active_clients: number;
  new_this_month: number;
  lost_this_month: number;
  pending_enrollments: number;
  by_plan_type: [string, number][];
  by_carrier: [string, number][];
  by_state: [string, number][];
  monthly_trend: MonthlyTrend[];
}

export interface MonthlyTrend {
  month: string;
  new_clients: number;
  lost_clients: number;
  net: number;
}

export interface EnrollmentListItem {
  id: string;
  client_name: string;
  plan_name?: string;
  carrier_name?: string;
  plan_type?: string;
  status?: string;
  effective_date?: string;
  termination_date?: string;
}

// ── Conversations ────────────────────────────────────────────────────────────

export type ConversationStatus = "OPEN" | "CLOSED" | "ARCHIVED";
export type EntryType = "CALL" | "EMAIL" | "MEETING" | "SMS" | "NOTE" | "SYSTEM";
export type CallDirection = "INBOUND" | "OUTBOUND";
export type CallOutcome = "ANSWERED" | "NO_ANSWER" | "VOICEMAIL" | "BUSY" | "CALLBACK_REQUESTED" | "WRONG_NUMBER";
export type MeetingType = "IN_PERSON" | "VIDEO" | "PHONE";

export interface Conversation {
  id: string;
  client_id: string;
  title: string;
  status: ConversationStatus;
  is_pinned: number;
  is_active: number;
  created_at?: string;
  updated_at?: string;
}

export interface ConversationListItem {
  id: string;
  client_id: string;
  title: string;
  status: ConversationStatus;
  is_pinned: number;
  entry_count: number;
  last_entry_at?: string;
  created_at?: string;
}

export interface ConversationEntry {
  id: string;
  conversation_id: string;
  client_id: string;
  entry_type: EntryType;
  subject?: string;
  body?: string;
  occurred_at?: string;
  follow_up_date?: string;
  follow_up_note?: string;
  call_direction?: CallDirection;
  call_duration?: number;
  call_outcome?: CallOutcome;
  call_phone_number?: string;
  meeting_location?: string;
  meeting_type?: MeetingType;
  email_to?: string;
  email_from?: string;
  system_event_type?: string;
  system_event_data?: string;
  is_active: number;
  created_at?: string;
  updated_at?: string;
}

export interface TimelineEntry {
  id: string;
  conversation_id: string;
  conversation_title: string;
  client_id: string;
  entry_type: EntryType;
  subject?: string;
  body?: string;
  occurred_at?: string;
  follow_up_date?: string;
  follow_up_note?: string;
  call_direction?: CallDirection;
  call_duration?: number;
  call_outcome?: CallOutcome;
  call_phone_number?: string;
  meeting_location?: string;
  meeting_type?: MeetingType;
  email_to?: string;
  email_from?: string;
  system_event_type?: string;
  system_event_data?: string;
  created_at?: string;
}

export interface CreateConversationInput {
  client_id: string;
  title: string;
}

export interface UpdateConversationInput {
  title?: string;
  status?: ConversationStatus;
  is_pinned?: number;
  is_active?: number;
}

export interface CreateConversationEntryInput {
  conversation_id: string;
  client_id: string;
  entry_type: EntryType;
  subject?: string;
  body?: string;
  occurred_at?: string;
  follow_up_date?: string;
  follow_up_note?: string;
  call_direction?: CallDirection;
  call_duration?: number;
  call_outcome?: CallOutcome;
  call_phone_number?: string;
  meeting_location?: string;
  meeting_type?: MeetingType;
  email_to?: string;
  email_from?: string;
}

export interface UpdateConversationEntryInput {
  subject?: string;
  body?: string;
  occurred_at?: string;
  follow_up_date?: string;
  follow_up_note?: string;
  call_direction?: CallDirection;
  call_duration?: number;
  call_outcome?: CallOutcome;
  call_phone_number?: string;
  meeting_location?: string;
  meeting_type?: MeetingType;
  email_to?: string;
  email_from?: string;
  is_active?: number;
}

// ── Carrier Sync ──────────────────────────────────────────────────────────────

export interface PortalMember {
  first_name: string;
  last_name: string;
  member_id?: string;
  dob?: string;
  plan_name?: string;
  effective_date?: string;
  end_date?: string;
  status?: string;
  policy_status?: string;
  state?: string;
  city?: string;
  phone?: string;
  email?: string;
}

export interface SyncResult {
  carrier_name: string;
  portal_count: number;
  local_count: number;
  matched: number;
  disenrolled: SyncDisenrollment[];
  new_in_portal: PortalMember[];
}

export interface SyncDisenrollment {
  client_name: string;
  client_id: string;
  enrollment_id: string;
  plan_name?: string;
}

export interface SyncLogEntry {
  id: string;
  carrier_id: string;
  carrier_name?: string;
  synced_at: string;
  portal_count: number;
  matched: number;
  disenrolled: number;
  new_found: number;
  status: string;
}
