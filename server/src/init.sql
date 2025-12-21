create table if not exists glm_requests (
  id uuid primary key,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  client_ip text not null,
  route text not null,
  status text not null,
  request_payload jsonb not null,
  glm_prompt text not null,
  glm_response text,
  error_text text,
  response_time_ms bigint
);

create index if not exists idx_glm_requests_status_created_at on glm_requests(status, created_at desc);

comment on table glm_requests is 'GLM 请求日志表：记录每次调用的请求、响应、状态与耗时';
comment on column glm_requests.id is '请求唯一标识（UUID）';
comment on column glm_requests.created_at is '创建时间（请求开始记录时）';
comment on column glm_requests.updated_at is '更新时间（请求结束/更新状态时）';
comment on column glm_requests.client_ip is '客户端 IP 地址';
comment on column glm_requests.route is '请求路由（例如 /generate）';
comment on column glm_requests.status is '请求状态（running/success/error）';
comment on column glm_requests.request_payload is '请求入参（JSON，已移除敏感字段）';
comment on column glm_requests.glm_prompt is '发送给模型的提示词（Prompt）';
comment on column glm_requests.glm_response is '模型原始返回内容（可能为空）';
comment on column glm_requests.error_text is '错误信息（失败时记录）';
comment on column glm_requests.response_time_ms is '响应耗时（毫秒）';
